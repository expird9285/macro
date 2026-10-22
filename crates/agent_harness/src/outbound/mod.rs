//! Outbound adapters: the concrete providers, stores, and carriers the domain
//! ports are satisfied by.

pub mod agent_prompt_composer;
pub mod channel_announcer;
pub mod channel_prompt_context;
/// Durable agent conversation turn journal.
pub mod conversation_turns;
pub mod egress;
pub mod forward;
pub mod notifications;
pub mod prompt_mentions;
pub mod session_tasks;
pub mod tool_approvals;
