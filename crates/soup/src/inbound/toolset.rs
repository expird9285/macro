//! Toolset inbound adapter for the Soup service.

mod list_entities;

#[cfg(test)]
mod test;

use crate::domain::ports::SoupService;
use ai_toolset::AsyncToolCollection;
use std::sync::Arc;
use uuid::Uuid;

pub use list_entities::{EntityItem, ItemType, ListEntities, ListEntitiesResponse, SortBy};

/// Service context for soup AI tools
pub struct SoupToolContext<T: SoupService> {
    /// The soup service instance
    pub service: Arc<T>,
    /// Entity id of the chat this request belongs to, when the request is an
    /// interactive chat session. `None` for every other feature, in which
    /// case nothing is excluded from the results.
    pub self_chat_id: Option<Uuid>,
}

impl<T: SoupService> Clone for SoupToolContext<T> {
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
            self_chat_id: self.self_chat_id,
        }
    }
}

impl<T: SoupService> SoupToolContext<T> {
    /// Create a new soup tool context
    pub fn new(service: T) -> Self {
        Self {
            service: Arc::new(service),
            self_chat_id: None,
        }
    }
}

/// Create a soup toolset
pub fn soup_toolset<T>() -> AsyncToolCollection<SoupToolContext<T>>
where
    T: SoupService,
{
    AsyncToolCollection::new().add_tool::<ListEntities, SoupToolContext<T>>()
}
