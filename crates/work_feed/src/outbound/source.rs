//! Feed candidates from the Soup service.

use models_soup::item::SoupItem;
use rootcause::Report;
use soup::domain::{
    models::{
        SoupProjectionHydration, WorkFeedPagePosition, WorkFeedSoupMode, WorkFeedSoupRequest,
    },
    ports::SoupService,
};

use crate::domain::{
    models::{WorkFeedMode, WorkFeedPosition},
    ports::{EntityFacts, SourceItem, SourcePage, SourceRequest, WorkFeedSource},
};

/// Candidates from Soup's work feed query, hydrated as Soup items.
pub struct SoupWorkFeedSource<S> {
    soup: S,
}

impl<S> SoupWorkFeedSource<S> {
    /// Create a source over the Soup service.
    pub fn new(soup: S) -> Self {
        Self { soup }
    }
}

/// The facts scoping and state need from a hydrated Soup item.
fn facts(item: &SoupItem<()>) -> EntityFacts {
    match item {
        SoupItem::ChannelThread(thread) => EntityFacts {
            thread_channel: Some(thread.channel_id),
        },
        _ => EntityFacts::default(),
    }
}

impl<S> WorkFeedSource for SoupWorkFeedSource<S>
where
    S: SoupService,
{
    type Entity = SoupProjectionHydration;

    async fn page(&self, request: SourceRequest) -> Result<SourcePage<Self::Entity>, Report> {
        let page = self
            .soup
            .get_work_feed_page(
                WorkFeedSoupRequest {
                    user: request.viewer.user.clone(),
                    limit: request.limit,
                    mode: match request.mode {
                        WorkFeedMode::Work => WorkFeedSoupMode::Work,
                        WorkFeedMode::Attention => WorkFeedSoupMode::Attention,
                    },
                    types: request.types,
                    filter: request.filter,
                    after: request.after.map(|after| WorkFeedPagePosition {
                        sort_at: after.sort_at,
                        entity_id: after.entity_id,
                    }),
                    only: request.only,
                },
                request.viewer.team,
            )
            .await
            .map_err(|error| rootcause::report!("failed to load work feed candidates: {error}"))?;

        Ok(SourcePage {
            items: page
                .items
                .into_iter()
                .map(|item| SourceItem {
                    key: item.hydration.item.entity(),
                    facts: facts(&item.hydration.item),
                    attention_at: item.attention_at,
                    touched_at: item.touched_at,
                    sort_at: item.sort_at,
                    entity: item.hydration,
                })
                .collect(),
            next: page.next.map(|next| WorkFeedPosition {
                sort_at: next.sort_at,
                entity_id: next.entity_id,
            }),
        })
    }
}
