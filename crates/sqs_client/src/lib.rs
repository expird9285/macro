mod message_attribute;
mod queue;

pub use queue::ReceivedMessage;

#[cfg(feature = "ai_projection")]
pub mod ai_projection;

#[cfg(feature = "chat")]
pub mod chat;

#[cfg(feature = "convert")]
pub mod convert;

#[cfg(feature = "document")]
pub mod document;

#[cfg(feature = "document_text_extractor")]
pub mod document_text_extractor;

#[cfg(feature = "organization_retention")]
mod organization_retention;

#[cfg(feature = "search")]
pub mod search;

#[cfg(feature = "upload_extractor")]
pub mod upload_extractor;

use aws_sdk_sqs as sqs;

pub const MAX_BATCH_SIZE: usize = 10;

/// Creates a deduplication id for the message
pub trait DeduplicationId {
    fn deduplication_id(&self) -> String;
}

/// Trait for generating the group id for the message.
/// This is typically just going to be the the same as PrimaryId to ensure messages are processed
/// in order relative to their PrimaryId.
pub trait GroupId {
    fn group_id(&self) -> String;
}

/// Gets the primary item id that we are performing the search operation on
/// This can be a document id, chat id, etc...
pub trait PrimaryId {
    fn id(&self) -> String;
}

#[derive(Clone, Debug)]
pub struct SQS {
    inner: sqs::Client,
    #[cfg(feature = "ai_projection")]
    ai_projection_queue: Option<String>,
    #[cfg(feature = "organization_retention")]
    organization_retention_queue: Option<String>,
    #[cfg(feature = "document")]
    document_delete_queue: Option<String>,
    #[cfg(feature = "chat")]
    chat_delete_queue: Option<String>,
    #[cfg(feature = "document_text_extractor")]
    document_text_extractor_queue: Option<String>,
    #[cfg(feature = "upload_extractor")]
    upload_extractor_queue: Option<String>,
    #[cfg(feature = "convert")]
    convert_queue: Option<String>,
    #[cfg(feature = "search")]
    search_event_queue: Option<String>,
}

impl SQS {
    pub fn new(inner: sqs::Client) -> Self {
        Self {
            inner,
            #[cfg(feature = "ai_projection")]
            ai_projection_queue: None,
            #[cfg(feature = "organization_retention")]
            organization_retention_queue: None,
            #[cfg(feature = "document")]
            document_delete_queue: None,
            #[cfg(feature = "chat")]
            chat_delete_queue: None,
            #[cfg(feature = "document_text_extractor")]
            document_text_extractor_queue: None,
            #[cfg(feature = "upload_extractor")]
            upload_extractor_queue: None,
            #[cfg(feature = "convert")]
            convert_queue: None,
            #[cfg(feature = "search")]
            search_event_queue: None,
        }
    }
}
