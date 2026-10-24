//! Toolset construction tests.
//!
//! Adding a tool to a toolset runs its input schema through
//! `generate_validated_input_schema` (via `AsyncToolObject::try_from_tool`),
//! which enforces the strict-mode requirements shared by OpenAI and
//! Anthropic. On a validation failure that path `.expect()`-panics, so a tool
//! with an unsupported schema (e.g. a `HashMap` that emits
//! `additionalProperties`) used to surface only at runtime when the service
//! built its toolset.
//!
//! These tests build every toolset the crate exposes. If any tool fails
//! schema validation, construction panics and the corresponding test fails —
//! turning that runtime failure into a test-time failure.

use super::*;
use ai_toolset::ToolSet as _;

#[test]
fn connector_discovery_is_eager_only_in_agent_sessions() {
    assert!(EAGER_TOOLS.contains(&"DiscoverConnectors"));
    assert!(
        tools_for(AiHost::AgentSession)
            .toolset
            .tools
            .contains_key("DiscoverConnectors")
    );
    for host in [AiHost::Chat, AiHost::ChannelBot, AiHost::Mcp] {
        assert!(
            !tools_for(host)
                .toolset
                .tools
                .contains_key("DiscoverConnectors")
        );
    }
}

#[test]
fn subagent_toolset_passes_schema_validation() {
    let tools = subagent_toolset();
    for name in [
        "ListDatabases",
        "DescribeDatabase",
        "QueryDatabase",
        "SaveDatabaseView",
    ] {
        assert!(
            tools.tools.contains_key(name),
            "delegated agents need {name}"
        );
    }
}

#[test]
fn database_only_toolset_exposes_exactly_its_database_capabilities() {
    let tools = database_tools();
    let names = tools
        .tools
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    let expected = [
        "ListDatabases",
        "DescribeDatabase",
        "QueryDatabase",
        "SaveDatabaseView",
        "DeleteDatabaseView",
        "SaveDatabaseQuery",
    ]
    .into_iter()
    .collect();
    assert_eq!(names, expected);
    assert!(
        tools.user_tools.is_empty(),
        "database actions must not expose user-tool composers"
    );
}

#[test]
fn every_host_toolset_passes_schema_validation() {
    for host in [
        AiHost::Chat,
        AiHost::AgentSession,
        AiHost::ChannelBot,
        AiHost::Mcp,
    ] {
        let tools = tools_for(host);
        assert!(
            tools.toolset.tools.contains_key("GenerateImage"),
            "{host:?} must expose image generation"
        );
    }
}

#[test]
fn project_workflows_are_available_in_every_host_alongside_folder_and_property_tools() {
    let names = [
        "ListInitiatives",
        "ReadInitiative",
        "CreateInitiative",
        "UpdateInitiative",
        "DeleteInitiative",
        "UpdateInitiativeSharing",
        "ReadInitiativeActivity",
        "SetEntityProperty",
        "ReadDiscussion",
        "Comment",
        "CommentOnText",
        "ResolveComment",
        "CreateProject",
        "ReadProject",
    ];
    for host in [
        AiHost::Chat,
        AiHost::AgentSession,
        AiHost::ChannelBot,
        AiHost::Mcp,
    ] {
        let json = frontend_schemas_builder()
            .merge(&tools_for(host))
            .build()
            .to_json_pretty()
            .expect("schemas serialize");
        let schema: serde_json::Value = serde_json::from_str(&json).unwrap();
        let tools = schema["tools"].as_array().unwrap();
        for name in names {
            assert!(
                tools.iter().any(|tool| tool["name"] == name),
                "{host:?} must expose {name}"
            );
        }
    }
}

/// Every host, including MCP, can take the item actions the app's menus
/// offer: editing channel messages and comments, trashing, duplicating,
/// renaming, sharing, favoriting, and muting items, DMs, calls, skills, and
/// routines.
#[test]
fn app_item_actions_are_available_in_every_host() {
    let names = [
        "EditChannelMessage",
        "DeleteChannelMessage",
        "ReactToChannelMessage",
        "OpenDirectMessage",
        "EditComment",
        "DeleteComment",
        "CreateSkill",
        "MoveToTrash",
        "DuplicateItem",
        "RenameItem",
        "UpdateSharing",
        "SetFavorite",
        "MuteNotifications",
        "RunRoutine",
        "DeleteRoutine",
        "UpdateCallRecord",
        "DeleteCallRecord",
    ];
    for host in [
        AiHost::Chat,
        AiHost::AgentSession,
        AiHost::ChannelBot,
        AiHost::Mcp,
    ] {
        let tools = tools_for(host);
        for name in names {
            assert!(
                tools.toolset.tools.contains_key(name),
                "{host:?} must expose {name}"
            );
        }
    }
}

/// Document answers get the one SQL tool, not a read-only twin: the access
/// they run over refuses the writes (see `databases_sql`'s view-only tests).
#[test]
fn document_answers_expose_discovery_and_the_one_query_tool() {
    let tools = database_read_only_tools();
    let names = tools
        .tools
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        names,
        ["ListDatabases", "DescribeDatabase", "QueryDatabase"]
            .into_iter()
            .collect()
    );
    assert_eq!(
        tools.tools["QueryDatabase"].annotations,
        database_tools().tools["QueryDatabase"].annotations,
        "the same QueryDatabase every host gets"
    );
    assert!(tools.user_tools.is_empty());
}

/// An agent session serves chat's tools, plus connector discovery.
#[test]
fn the_agent_session_host_has_chats_tools_plus_connector_discovery() {
    let session = tools_for(AiHost::AgentSession);
    assert_eq!(
        session.toolset.request_schemas().map(|schemas| schemas
            .into_iter()
            .filter(|schema| schema.name != "DiscoverConnectors")
            .count()),
        tools_for(AiHost::Chat)
            .toolset
            .request_schemas()
            .map(|schemas| schemas.len()),
        "the same tools as chat except agent-session connector discovery"
    );
}

#[test]
fn no_tools_passes_schema_validation() {
    let _ = no_tools();
}

#[test]
fn search_toolset_passes_schema_validation() {
    let _ = search_toolset();
}

#[test]
fn frontend_schemas_build() {
    let json = all_tool_frontend_schemas().to_json_pretty().unwrap();
    let schemas: serde_json::Value = serde_json::from_str(&json).unwrap();
    let mut names = std::collections::HashSet::new();
    for tool in schemas["tools"].as_array().unwrap() {
        let name = tool["name"].as_str().unwrap();
        assert!(names.insert(name), "duplicate frontend tool: {name}");
    }
    assert!(names.contains("QueryDatabase"));
    assert!(names.contains("DiscoverConnectors"));
    for name in tools_for(AiHost::Chat).toolset.tools.keys() {
        assert!(names.contains(name.as_str()), "missing chat tool: {name}");
    }
    for name in [
        "CreateDatabase",
        "CreateTable",
        "RenameDatabase",
        "RenameTable",
        "ReorderTables",
        "DeleteTable",
        "AddColumn",
        "AddColumnOptions",
        "RenameColumn",
        "ChangeColumnType",
        "DeleteColumn",
        "ReorderColumns",
    ] {
        assert!(
            !names.contains(name),
            "removed tool {name} must not be generated"
        );
    }
}

/// The configure-agent system skill walks an agent through `ListAgents` and
/// `ConfigureAgent`; a host that reads the skill must be able to follow it.
#[test]
fn every_host_exposes_agent_configuration() {
    for host in [
        AiHost::Chat,
        AiHost::AgentSession,
        AiHost::ChannelBot,
        AiHost::Mcp,
    ] {
        let tools = tools_for(host);
        for name in ["ListAgents", "ConfigureAgent", "ConfigureBot"] {
            assert!(
                tools.toolset.tools.contains_key(name),
                "{host:?} missing {name}"
            );
        }
    }
}

#[test]
fn every_host_exposes_skill_discovery_and_reading() {
    for host in [
        AiHost::Chat,
        AiHost::AgentSession,
        AiHost::ChannelBot,
        AiHost::Mcp,
    ] {
        let tools = tools_for(host);
        for name in ["ListSkills", "SearchSkills", "ReadSkill"] {
            assert!(
                tools.toolset.tools.contains_key(name),
                "{host:?} missing {name}"
            );
        }
        assert!(
            tools.prompt.to_string().contains("ReadSkill"),
            "{host:?} missing skill reading instructions"
        );
    }
}

#[test]
fn hosts_with_tool_search_defer_all_but_the_core_tools() {
    for host in [AiHost::Chat, AiHost::AgentSession, AiHost::ChannelBot] {
        let tools = tools_for(host);
        let lazy = DeferredToolSet::new(tools.toolset.clone(), tools.deferred.clone());
        let sent: Vec<String> = lazy
            .request_schemas()
            .unwrap_or_default()
            .into_iter()
            .map(|schema| schema.name)
            .collect();
        let catalog: Vec<String> = lazy
            .searchable_catalog()
            .into_iter()
            .map(|tool| tool.name)
            .collect();
        let prompt = tools.prompt.to_string();

        for name in ["ReadContent", "ContentSearch", "LoadTools", "SearchTools"] {
            assert!(sent.contains(&name.to_string()), "{host:?} sends {name}");
        }
        for name in ["CommentOnText", "QueryDatabase"] {
            assert!(!sent.contains(&name.to_string()), "{host:?} defers {name}");
            assert!(
                catalog.contains(&name.to_string()),
                "{host:?} catalogs {name}"
            );
            assert!(
                prompt.contains(&format!("\n- {name}: ")),
                "{host:?} lists {name} in the prompt"
            );
        }
        assert_eq!(
            sent.len() + catalog.len(),
            tools.toolset.tools.len(),
            "{host:?}: every tool is either sent or catalogued"
        );
    }
}

#[test]
fn the_mcp_host_defers_nothing() {
    let tools = tools_for(AiHost::Mcp);

    assert!(tools.deferred.is_empty());
    assert!(!tools.prompt.to_string().contains("LoadTools"));
}

/// Only an MCP client can PUT bytes to the upload URL, so the tool is not
/// offered to in-app hosts, which would call it and then have no way to use it.
#[test]
fn channel_media_uploads_are_mcp_only() {
    assert!(
        tools_for(AiHost::Mcp)
            .toolset
            .tools
            .contains_key("CreateChannelMediaUpload")
    );
    for host in [AiHost::Chat, AiHost::AgentSession, AiHost::ChannelBot] {
        assert!(
            !tools_for(host)
                .toolset
                .tools
                .contains_key("CreateChannelMediaUpload"),
            "{host:?}"
        );
    }
}

#[test]
fn every_eager_tool_exists() {
    let tools = tools_for(AiHost::AgentSession);
    for name in EAGER_TOOLS {
        assert!(
            tools.toolset.tools.contains_key(*name),
            "{name} is not a tool"
        );
    }
}

#[test]
fn forms_are_discoverable_and_execute_directly_on_every_host() {
    for host in [
        AiHost::Chat,
        AiHost::AgentSession,
        AiHost::ChannelBot,
        AiHost::Mcp,
    ] {
        let tools = tools_for(host);
        for name in [
            "CreateForm",
            "ReadForm",
            "EditForm",
            "ListForms",
            "SetFormAccess",
        ] {
            assert!(tools.toolset.tools.contains_key(name), "{host:?}: {name}");
            if !matches!(host, AiHost::Mcp) {
                assert!(tools.prompt.to_string().contains(name), "{host:?}: {name}");
            }
        }
        assert!(!tools.toolset.user_tools.contains_key("SetFormAccess"));
    }
    let delegated = subagent_toolset();
    for name in ["CreateForm", "ReadForm", "EditForm", "ListForms"] {
        assert!(delegated.tools.contains_key(name));
    }
    assert!(!delegated.tools.contains_key("SetFormAccess"));
}

#[test]
fn code_mode_retains_direct_lifecycles_and_hides_sdk_methods_from_lazy_discovery() {
    let standard = tools_for(AiHost::AgentSession);
    let direct = tools_for_agent_code_mode();
    let sdk = code_mode_tools();
    assert!(direct.toolset.tools.len() < standard.toolset.tools.len());
    for name in ["ReadContent"] {
        assert!(standard.toolset.tools.contains_key(name), "{name}");
        assert!(sdk.tools.contains_key(name), "{name}");
        assert!(!direct.toolset.tools.contains_key(name), "{name}");
        assert!(!direct.deferred.iter().any(|t| t.name == name), "{name}");
    }
    for name in [
        "DisplayResults",
        "NameSearch",
        "ListEntities",
        "ContentSearch",
        "CreateDocument",
        "ListSkills",
        "SearchSkills",
        "ReadSkill",
        "GenerateImage",
        "CreateRoutine",
        "EditDocument",
        "Subagent",
        "SearchTools",
        "LoadTools",
        "DiscoverConnectors",
        "BashCodeExecution",
        "TextEditorCodeExecution",
    ] {
        assert!(direct.toolset.tools.contains_key(name), "{name}");
    }
    for name in standard.toolset.user_tools.keys() {
        assert!(direct.toolset.user_tools.contains_key(name), "{name}");
        assert!(direct.toolset.tools.contains_key(name), "{name}");
    }
    let standard_deferred =
        DeferredToolSet::new(standard.toolset.clone(), standard.deferred.clone());
    assert!(
        standard_deferred
            .request_schemas()
            .unwrap()
            .iter()
            .any(|schema| schema.name == "ListEntities")
    );
    let deferred = DeferredToolSet::new(direct.toolset.clone(), direct.deferred.clone());
    let eager = deferred.request_schemas().unwrap();
    for name in [
        "ListEntities",
        "NameSearch",
        "ContentSearch",
        "EditDocument",
    ] {
        assert!(eager.iter().any(|schema| schema.name == name), "{name}");
    }
    let prompt = direct.prompt.to_string();
    assert!(!prompt.contains("- ReadMetadata:"));
    let routing = prompt::tool_usage::CODE_MODE_ROUTING.to_string();
    assert!(prompt.contains(&routing));
    assert!(!standard.prompt.to_string().contains(&routing));
    for host in [AiHost::Chat, AiHost::ChannelBot, AiHost::Mcp] {
        assert!(!tools_for(host).prompt.to_string().contains(&routing));
    }
}

#[test]
fn session_reader_is_available_to_every_conversation_host_and_mcp() {
    for host in [
        AiHost::Chat,
        AiHost::AgentSession,
        AiHost::ChannelBot,
        AiHost::Mcp,
    ] {
        let tools = tools_for(host);
        assert!(tools.toolset.tools.contains_key("ReadAgentSession"));
        assert!(
            !tools
                .deferred
                .iter()
                .any(|tool| tool.name == "ReadAgentSession")
        );
    }
    assert!(subagent_toolset().tools.contains_key("ReadAgentSession"));
    let code_mode = tools_for_agent_code_mode();
    assert!(code_mode.toolset.tools.contains_key("ReadAgentSession"));
    assert!(
        !code_mode
            .deferred
            .iter()
            .any(|tool| tool.name == "ReadAgentSession")
    );
}

#[test]
fn conversation_tools_only_advertise_new_sessions() {
    for tools in [
        tools_for(AiHost::Chat),
        tools_for(AiHost::AgentSession),
        tools_for(AiHost::ChannelBot),
        tools_for(AiHost::Mcp),
        tools_for_agent_code_mode(),
    ] {
        assert!(!tools.toolset.tools.contains_key("ReadChat"));
        assert!(!tools.prompt.to_string().contains("ReadChat"));
        for name in [
            "ReadAgentSession",
            "ReadAgentSessionSharing",
            "UpdateAgentSessionSharing",
            "CreateAgentSession",
            "ManageAgentSession",
        ] {
            assert!(tools.toolset.tools.contains_key(name), "missing {name}");
        }
        for schema in tools.toolset.request_schemas().unwrap() {
            let json = serde_json::to_string(&schema.schema).unwrap();
            if [
                "NameSearch",
                "ContentSearch",
                "ListEntities",
                "RenameItem",
                "MoveToTrash",
                "MoveToProject",
                "DuplicateItem",
                "UpdateSharing",
                "ShareWithChannel",
                "SetFavorite",
                "GetEntityProperties",
                "SetEntityProperty",
                "BulkSetEntityPropertyOptions",
                "ListNotifications",
            ]
            .contains(&schema.name.as_str())
            {
                assert!(
                    !json.contains("\"chat\""),
                    "{} advertises legacy chats",
                    schema.name
                );
                assert!(
                    !json.contains("\"chats\""),
                    "{} advertises legacy chat search",
                    schema.name
                );
                assert!(
                    !json.contains("\"ai_chat\""),
                    "{} advertises legacy chat listing",
                    schema.name
                );
            }
        }
    }
    assert!(!subagent_toolset().tools.contains_key("ReadChat"));
}
