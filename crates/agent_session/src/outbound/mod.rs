//! Adapters implementing the domain ports for external systems.

pub mod postgres;

/// Haiku-backed automatic session naming.
pub mod name_generator;

/// Streaming a live session's log to a channel's viewers.
pub mod connection_gateway_realtime;

/// Publishing lifecycle facts to the event broker.
pub mod broker_lifecycle_publisher;

/// Internal HTTP client for agent session creation.
pub mod coding_agents_client;

/// Internal HTTP client for existing agent session actions.
pub mod session_actions_client;
