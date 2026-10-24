//! Outbound adapters for channel bot dependencies.

mod agent_loop_responder;
/// Current parent capabilities for built-in agent replies.
pub mod conversation;
mod fast_model_trigger_classifier;
mod lexical_comment_marks;

pub use agent_loop_responder::AgentLoopResponder;
pub use fast_model_trigger_classifier::FastModelTriggerClassifier;
pub use lexical_comment_marks::LexicalCommentMarks;
