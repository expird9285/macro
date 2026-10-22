use agent_runtime_protocol::domain::action::AgentAction;
use agent_session::domain::model::{AgentMcpServers, AgentSessionId};
use agent_trigger::domain::broker_events::{
    AgentAssignedToTaskEvent, AgentBotMentionedEvent, AgentConversationMessageEvent,
    AgentMentionedEvent, AgentTriggerTopicEvent, ChannelEventMetadata, ExistingAgentSessionEvent,
    NewAgentSessionEvent, ThreadEventMetadata, ThreadMessageKind,
};
use bot_id::{BotId, MACRO_CODER_BOT_ID};
use channel_sender::ChannelSender;
use channels::domain::broker_events::ChannelMessagePostedMetadata;
use channels::domain::models::ChannelType;
use chrono::Utc;
use macro_user_id::user_id::MacroUserIdStr;
use macro_uuid::Uuid;
use messages::domain::events::MessagePostedMetadata;
use messages::domain::models::MessageParent;
use trigger_context::{
    AddressedBy, ContextPerson, DiscussionContext, DiscussionSurface, FollowUpContext, ReplyTarget,
    TaskAssignedContext, TaskSnapshot, TriggerContext,
};

use super::*;
use crate::domain::model::{
    AgentKind, AgentRuntimeConfig, HarnessCommand, MentionOrigin, OpenSession, SessionOrigin,
    StaticFileLinks,
};
use agent_runtime_protocol::domain::action::PromptAttachment;
use channels::domain::broker_events::ChannelEventAttachment;

fn mention_origin(open: &OpenSession) -> &MentionOrigin {
    let SessionOrigin::Mention(origin) = &open.origin else {
        panic!("expected a mention origin");
    };
    origin
}

fn runtime(kind: AgentKind) -> Option<AgentRuntimeConfig> {
    Some(AgentRuntimeConfig {
        kind,
        model: "configured-model".to_owned(),
        harness: "in-memory".to_owned(),
        instructions: "configured instructions".to_owned(),
        mcp_servers: AgentMcpServers::OwnerConnections,
    })
}

fn links() -> StaticFileLinks {
    StaticFileLinks::new("https://static.example/")
}

fn user() -> MacroUserIdStr<'static> {
    MacroUserIdStr::try_from_email("asker@macro.com").expect("a valid user id")
}

/// A channel post in the channel-only shape channel triggers travel in.
fn message(sender: ChannelSender<'static>) -> ChannelMessagePostedMetadata {
    ChannelMessagePostedMetadata {
        channel_id: Uuid::from_u128(1),
        message_id: Uuid::from_u128(2),
        thread_id: None,
        sender,
        triggered_by: None,
        channel_type: ChannelType::Public,
        content: "@claude fix the tests".to_owned(),
        mentions: vec![],
        attachments: vec![],
        created_at: Utc::now(),
    }
}

fn document() -> MessageParent {
    MessageParent::parse("document", "doc-1").unwrap()
}

/// A document discussion post, which only the parent-aware shape carries.
fn document_message(sender: ChannelSender<'static>) -> MessagePostedMetadata {
    MessagePostedMetadata {
        parent: document(),
        message_id: Uuid::from_u128(2),
        thread_id: None,
        root_id: Uuid::from_u128(2),
        sender,
        triggered_by: None,
        content: "@claude fix the tests".to_owned(),
        mentions: vec![],
        attachments: vec![],
        created_at: Utc::now(),
    }
}

fn mentioned(bot: BotId, sender: ChannelSender<'static>) -> AgentTriggerTopicEvent {
    AgentTriggerTopicEvent::New(NewAgentSessionEvent::TopLevelMentioned(
        AgentBotMentionedEvent {
            bot_id: bot,
            message: message(sender),
            context: None,
        },
    ))
}

fn channel_message(bot: BotId) -> AgentTriggerTopicEvent {
    AgentTriggerTopicEvent::Existing(ExistingAgentSessionEvent::Channel(ChannelEventMetadata {
        bot_id: bot,
        session_id: AgentSessionId::TEST_A,
        kind: ThreadMessageKind::MentionThread,
        message: message(ChannelSender::new_from_user(user())),
        context: None,
    }))
}

fn channel_message_from(bot: BotId, sender: ChannelSender<'static>) -> AgentTriggerTopicEvent {
    AgentTriggerTopicEvent::Existing(ExistingAgentSessionEvent::Channel(ChannelEventMetadata {
        bot_id: bot,
        session_id: AgentSessionId::TEST_A,
        kind: ThreadMessageKind::MentionThread,
        message: message(sender),
        context: None,
    }))
}

fn assigned_to_task() -> AgentTriggerTopicEvent {
    AgentTriggerTopicEvent::New(NewAgentSessionEvent::AssignedToTask(
        AgentAssignedToTaskEvent {
            parent: document(),
            discussion_id: Uuid::from_u128(3),
            bot_id: BotId::TEST_A,
            actor: user(),
            prompt: "Refactor the parser".to_owned(),
            context: None,
        },
    ))
}

#[test]
fn a_mention_for_a_managed_bot_opens_a_new_session() {
    let event = mentioned(BotId::TEST_A, ChannelSender::new_from_user(user()));
    let routed = route_agent_trigger(
        event,
        runtime(AgentKind::InMemory),
        &links(),
    )
    .expect("a top-level mention should yield work");

    let RoutedTrigger::Command(_, HarnessCommand::Open(open)) = routed else {
        panic!("a managed mention should open");
    };
    assert_eq!(open.bot_id, BotId::TEST_A);
    assert_eq!(open.runtime.kind, AgentKind::InMemory);
    let origin = mention_origin(&open);
    assert_eq!(origin.sender, user());
    assert_eq!(origin.content, "@claude fix the tests");
    assert_eq!(
        origin.parent,
        messages::domain::models::MessageParent::Channel(Uuid::from_u128(1))
    );
    assert_eq!(origin.thread_id, Uuid::from_u128(2));
    assert_eq!(origin.message_id, Uuid::from_u128(2));
}

#[test]
fn a_managed_channel_message_forwards_to_its_session() {
    let routed = route_agent_trigger(
        channel_message(MACRO_CODER_BOT_ID),
        runtime(AgentKind::InMemory),
        &links(),
    )
    .expect("a channel event for our bot should yield work");

    let RoutedTrigger::Command(session_id, HarnessCommand::Deliver(deliver)) = routed else {
        panic!("a managed existing-session event should deliver");
    };
    assert_eq!(session_id, AgentSessionId::TEST_A);
    assert_eq!(deliver.actor, Some(user()));
    assert_eq!(
        deliver.action,
        AgentAction::prompt("@claude fix the tests"),
        "a channel message becomes a prompt"
    );
    let announce = deliver.announce.expect("a channel prompt offers an origin");
    assert_eq!(
        announce.parent,
        messages::domain::models::MessageParent::Channel(Uuid::from_u128(1))
    );
    assert_eq!(announce.thread_id, Uuid::from_u128(2));
    assert_eq!(announce.message_id, Uuid::from_u128(2));
}

#[test]
fn a_mention_with_attached_files_opens_with_them_as_prompt_attachments() {
    let event = AgentTriggerTopicEvent::New(NewAgentSessionEvent::TopLevelMentioned(
        AgentBotMentionedEvent {
            bot_id: BotId::TEST_A,
            message: message_with_files(ChannelSender::new_from_user(user())),
            context: None,
        },
    ));
    let routed = route_agent_trigger(event, runtime(AgentKind::InMemory), &links()).unwrap();
    let RoutedTrigger::Command(_, HarnessCommand::Open(open)) = routed else {
        panic!("a new-session event should open");
    };
    assert_eq!(
        mention_origin(&open).attachments,
        vec![
            PromptAttachment::new(
                format!("https://static.example/file/{}", Uuid::from_u128(0x10)),
                "image",
            )
            .mime_type("image/*"),
            PromptAttachment::new(
                format!("https://static.example/file/{}", Uuid::from_u128(0x12)),
                "video",
            )
            .mime_type("video/*"),
        ]
    );
}

fn attached(entity_type: &str, id: u128) -> ChannelEventAttachment {
    ChannelEventAttachment {
        attachment_id: Uuid::from_u128(id),
        entity_type: entity_type.to_owned(),
        entity_id: Uuid::from_u128(id).to_string(),
        created_at: Utc::now(),
    }
}

fn message_with_files(sender: ChannelSender<'static>) -> ChannelMessagePostedMetadata {
    ChannelMessagePostedMetadata {
        attachments: vec![
            attached("static/image", 0x10),
            attached("document", 0x11),
            attached("static/video", 0x12),
        ],
        ..message(sender)
    }
}

#[test]
fn a_managed_channel_message_with_files_delivers_them_as_prompt_attachments() {
    let event = AgentTriggerTopicEvent::Existing(ExistingAgentSessionEvent::Channel(
        ChannelEventMetadata {
            bot_id: MACRO_CODER_BOT_ID,
            session_id: AgentSessionId::TEST_A,
            kind: ThreadMessageKind::MentionThread,
            message: message_with_files(ChannelSender::new_from_user(user())),
            context: None,
        },
    ));
    let routed = route_agent_trigger(event, runtime(AgentKind::InMemory), &links()).unwrap();
    let RoutedTrigger::Command(_, HarnessCommand::Deliver(deliver)) = routed else {
        panic!("a managed existing-session event should deliver");
    };
    let AgentAction::Prompt(prompt) = deliver.action else {
        panic!("a channel message becomes a prompt");
    };
    assert_eq!(prompt.prompt, "@claude fix the tests");
    assert_eq!(
        prompt.attachments.len(),
        2,
        "the document is not a file link"
    );
    assert_eq!(prompt.attachments[0].mime_type.as_deref(), Some("image/*"));
    assert_eq!(prompt.attachments[1].mime_type.as_deref(), Some("video/*"));
}

#[test]
fn conversation_messages_route_to_every_configured_runtime() {
    let session_id = AgentSessionId::TEST_A;
    let event = AgentTriggerTopicEvent::New(NewAgentSessionEvent::ConversationMessage(
        AgentConversationMessageEvent {
            bot_id: BotId::TEST_A,
            session_id,
            message: document_message(ChannelSender::new_from_user(user())),
            context: None,
        },
    ));
    assert_eq!(
        route_agent_trigger(event.clone(), None, &links()).unwrap_err(),
        Skipped::ForeignBot,
    );
    let routed = route_agent_trigger(event.clone(), runtime(AgentKind::InMemory), &links()).unwrap();
    let RoutedTrigger::Command(id, HarnessCommand::ConversationMessage(open)) = routed else {
        panic!("a conversation message should reach its reserved session");
    };
    assert_eq!(id, session_id);
    assert_eq!(open.runtime.kind, AgentKind::InMemory);
}
