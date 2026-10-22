//! Macro Internal MCP tools for agent sessions.

mod coding_agents;
mod context;
mod link_task;
mod manage_agent_session;
mod read_agent_session;
mod set_pull_request;
mod task_pull_requests;

pub use coding_agents::{CodingAgentToolContext, CreateAgentSession, coding_agent_toolset};
pub use context::SessionToolContext;
pub use link_task::LinkTask;
pub use manage_agent_session::ManageAgentSession;
pub use read_agent_session::{ReadAgentSession, ReadAgentSessionContext};
pub use set_pull_request::SetPullRequest;
pub use task_pull_requests::TaskPullRequestsTool;

mod sharing;
pub use sharing::{ReadAgentSessionSharing, UpdateAgentSessionSharing};
