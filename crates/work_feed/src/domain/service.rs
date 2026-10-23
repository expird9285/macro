//! The work feed use cases: list a page, recompute known items, mark items
//! done, and undo a done.

use std::collections::{HashMap, HashSet};

use model_entity::{Entity, EntityType};
use rootcause::Report;

use super::{
    models::{
        FeedNotification, WorkFeedChange, WorkFeedDoneOutcome, WorkFeedDoneReceipt,
        WorkFeedDoneTarget, WorkFeedEntry, WorkFeedError, WorkFeedItem, WorkFeedItemKey,
        WorkFeedItemState, WorkFeedPage, WorkFeedPrimaryReason, WorkFeedQuery, WorkFeedScope,
        WorkFeedViewer, counts_as_unseen,
    },
    policy,
    ports::{SourceItem, SourceRequest, WorkFeedNotifications, WorkFeedSignals, WorkFeedSource},
    stacking,
};

#[cfg(test)]
mod test;

/// Most candidate keys recomputed per source request.
const RECOMPUTE_CHUNK: usize = 200;

/// The work feed's use cases.
pub trait WorkFeedService: Send + Sync + 'static {
    /// The hydrated entity each item carries.
    type Entity: Send + Sync + 'static;

    /// One page of the viewer's feed, newest first.
    fn page(
        &self,
        viewer: WorkFeedViewer,
        query: WorkFeedQuery,
    ) -> impl Future<Output = Result<WorkFeedPage<Self::Entity>, WorkFeedError>> + Send;

    /// The current membership and content of known items in one feed: each
    /// key comes back upserted with its placement, or removed.
    fn recompute(
        &self,
        viewer: WorkFeedViewer,
        scope: WorkFeedScope,
        keys: Vec<WorkFeedItemKey>,
    ) -> impl Future<Output = Result<Vec<WorkFeedChange<Self::Entity>>, WorkFeedError>> + Send;

    /// Acknowledge the attention the caller observed on each item: its
    /// notifications move to done. Own work
    /// is not acknowledged, so an item the viewer also worked on stays in the
    /// work feed, seen, at its own-work time. Underlying tasks, discussions
    /// and pull requests are not completed.
    fn mark_done(
        &self,
        viewer: WorkFeedViewer,
        targets: Vec<WorkFeedDoneTarget>,
    ) -> impl Future<Output = Result<WorkFeedDoneOutcome, WorkFeedError>> + Send;

    /// Reverse exactly what one mark-done changed, then recompute its items
    /// for the caller's feed.
    fn undo_done(
        &self,
        viewer: WorkFeedViewer,
        scope: WorkFeedScope,
        receipt: WorkFeedDoneReceipt,
    ) -> impl Future<Output = Result<Vec<WorkFeedChange<Self::Entity>>, WorkFeedError>> + Send;
}

/// The work feed service over its ports.
pub struct WorkFeedServiceImpl<S, N, G> {
    source: S,
    notifications: N,
    signals: G,
}

impl<S, N, G> WorkFeedServiceImpl<S, N, G> {
    /// Create the service from its ports.
    pub fn new(source: S, notifications: N, signals: G) -> Self {
        Self {
            source,
            notifications,
            signals,
        }
    }
}

/// The notifications an item's scope is drawn from: a thread shares its
/// channel's notifications, every other item reads its own entity's.
fn notification_source(item: &SourceItem<impl Sized>) -> Option<Entity<'static>> {
    match item.key.entity_type {
        EntityType::ChannelMessage => item
            .facts
            .thread_channel
            .map(|channel| EntityType::Channel.with_entity_string(channel.to_string())),
        _ => Some(item.key.clone()),
    }
}

/// Build one item from its candidate and its source's active notifications.
/// Returns `None` when, after scoping, the item has no reason left.
fn assemble_item<E>(
    item: SourceItem<E>,
    loaded: &HashMap<Entity<'static>, Vec<FeedNotification>>,
) -> Option<WorkFeedEntry<E>> {
    let source = notification_source(&item);
    let available = source
        .as_ref()
        .and_then(|source| loaded.get(source))
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut scoped = match item.key.entity_type {
        EntityType::Channel => stacking::scope_channel(available),
        EntityType::ChannelMessage => {
            stacking::scope_channel_thread(available, item.key.entity_id.as_ref())
        }
        _ => available.to_vec(),
    };
    stacking::sort_newest_first(&mut scoped);
    if scoped.is_empty() && item.touched_at.is_none() {
        return None;
    }

    let stacks = stacking::stack(&scoped);
    let attention_at = scoped.first().map(|notification| notification.created_at);
    let unseen = scoped
        .iter()
        .any(|notification| counts_as_unseen(notification));
    let primary = match (attention_at, item.touched_at) {
        (Some(attention), Some(touched)) if touched >= attention => WorkFeedPrimaryReason::OwnWork,
        (Some(_), _) if !stacks.is_empty() => WorkFeedPrimaryReason::Attention { stack: 0 },
        _ => WorkFeedPrimaryReason::OwnWork,
    };

    Some(WorkFeedEntry {
        sort_at: item.sort_at,
        item: WorkFeedItem {
            key: WorkFeedItemKey::new(item.key),
            entity: item.entity,
            state: if unseen {
                WorkFeedItemState::Unseen
            } else {
                WorkFeedItemState::Seen
            },
            attention_at,
            touched_at: item.touched_at,
            notifications: scoped,
            stacks,
            primary,
        },
    })
}

impl<S, N, G> WorkFeedServiceImpl<S, N, G>
where
    S: WorkFeedSource,
    N: WorkFeedNotifications,
    G: WorkFeedSignals,
{
    /// Load the candidates' notifications and build their items, in order.
    async fn assemble(
        &self,
        viewer: &WorkFeedViewer,
        items: Vec<SourceItem<S::Entity>>,
    ) -> Result<Vec<WorkFeedEntry<S::Entity>>, Report> {
        let mut sources = Vec::new();
        let mut seen = HashSet::new();
        for source in items.iter().filter_map(notification_source) {
            if seen.insert(source.clone()) {
                sources.push(source);
            }
        }
        let loaded = if sources.is_empty() {
            HashMap::new()
        } else {
            self.notifications
                .active(viewer.user.clone(), sources)
                .await?
        };
        Ok(items
            .into_iter()
            .filter_map(|item| assemble_item(item, &loaded))
            .collect())
    }

    /// Current items for `keys` in `scope`, keyed by item.
    async fn current(
        &self,
        viewer: &WorkFeedViewer,
        scope: &WorkFeedScope,
        keys: &[WorkFeedItemKey],
    ) -> Result<HashMap<WorkFeedItemKey, WorkFeedEntry<S::Entity>>, Report> {
        let filter = policy::signal_filter(&viewer.user, scope.include_snippets);
        let types = policy::candidate_types(&scope.types);
        let mut current = HashMap::with_capacity(keys.len());
        for chunk in keys.chunks(RECOMPUTE_CHUNK) {
            let page = self
                .source
                .page(SourceRequest {
                    viewer: viewer.clone(),
                    mode: scope.mode,
                    types: types.clone(),
                    filter: filter.clone(),
                    limit: u16::try_from(chunk.len()).unwrap_or(u16::MAX),
                    after: None,
                    only: Some(chunk.iter().map(|key| key.entity().clone()).collect()),
                })
                .await?;
            for entry in self.assemble(viewer, page.items).await? {
                current.insert(entry.item.key.clone(), entry);
            }
        }
        Ok(current)
    }
}

/// Keys deduplicated in first-seen order.
fn unique_keys(keys: impl IntoIterator<Item = WorkFeedItemKey>) -> Vec<WorkFeedItemKey> {
    let mut seen = HashSet::new();
    keys.into_iter()
        .filter(|key| seen.insert(key.clone()))
        .collect()
}

impl<S, N, G> WorkFeedService for WorkFeedServiceImpl<S, N, G>
where
    S: WorkFeedSource,
    N: WorkFeedNotifications,
    G: WorkFeedSignals,
{
    type Entity = S::Entity;

    #[tracing::instrument(err, skip(self, viewer), fields(user = %viewer.user))]
    async fn page(
        &self,
        viewer: WorkFeedViewer,
        query: WorkFeedQuery,
    ) -> Result<WorkFeedPage<Self::Entity>, WorkFeedError> {
        let page = self
            .source
            .page(SourceRequest {
                viewer: viewer.clone(),
                mode: query.scope.mode,
                types: policy::candidate_types(&query.scope.types),
                filter: policy::signal_filter(&viewer.user, query.scope.include_snippets),
                limit: query.limit,
                after: query.after,
                only: None,
            })
            .await?;
        let entries = self.assemble(&viewer, page.items).await?;
        Ok(WorkFeedPage {
            entries,
            next: page.next,
        })
    }

    #[tracing::instrument(err, skip(self, viewer, keys), fields(user = %viewer.user, keys = keys.len()))]
    async fn recompute(
        &self,
        viewer: WorkFeedViewer,
        scope: WorkFeedScope,
        keys: Vec<WorkFeedItemKey>,
    ) -> Result<Vec<WorkFeedChange<Self::Entity>>, WorkFeedError> {
        let keys = unique_keys(keys);
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        let mut current = self.current(&viewer, &scope, &keys).await?;
        Ok(keys
            .into_iter()
            .map(|key| match current.remove(&key) {
                Some(entry) => WorkFeedChange::Upserted(entry),
                None => WorkFeedChange::Removed(key),
            })
            .collect())
    }

    #[tracing::instrument(err, skip(self, viewer, targets), fields(user = %viewer.user, targets = targets.len()))]
    async fn mark_done(
        &self,
        viewer: WorkFeedViewer,
        targets: Vec<WorkFeedDoneTarget>,
    ) -> Result<WorkFeedDoneOutcome, WorkFeedError> {
        let keys = unique_keys(targets.iter().map(|target| target.key.clone()));
        if keys.is_empty() {
            return Ok(WorkFeedDoneOutcome {
                items: Vec::new(),
                receipt: WorkFeedDoneReceipt::default(),
            });
        }
        // Evaluate each item in every feed's scope, whatever feed the caller
        // was looking at, so done acknowledges what they observed.
        let current = self
            .current(&viewer, &WorkFeedScope::everything(), &keys)
            .await?;

        let mut notification_ids = Vec::new();
        let mut planned = HashSet::new();
        for target in &targets {
            if !planned.insert(target.key.clone()) {
                continue;
            }
            let Some(entry) = current.get(&target.key) else {
                // Already gone from every feed: nothing left to acknowledge.
                continue;
            };
            let item = &entry.item;
            let observed = target.revision.unwrap_or_else(|| item.revision());
            if let Some(through) = observed.attention_through {
                notification_ids.extend(
                    item.notifications
                        .iter()
                        .filter(|notification| notification.created_at <= through)
                        .map(|notification| notification.notification_id),
                );
            }
        }

        let user = viewer.user.clone();
        let changed_notifications = if notification_ids.is_empty() {
            Vec::new()
        } else {
            self.notifications
                .set_done(user.clone(), notification_ids, true)
                .await?
        };

        self.signals
            .items_changed(user, keys.iter().map(|key| key.entity().clone()).collect())
            .await;

        Ok(WorkFeedDoneOutcome {
            receipt: WorkFeedDoneReceipt {
                items: keys.iter().map(WorkFeedItemKey::id).collect(),
                notification_ids: changed_notifications,
            },
            items: keys,
        })
    }

    #[tracing::instrument(err, skip(self, viewer, receipt), fields(user = %viewer.user))]
    async fn undo_done(
        &self,
        viewer: WorkFeedViewer,
        scope: WorkFeedScope,
        receipt: WorkFeedDoneReceipt,
    ) -> Result<Vec<WorkFeedChange<Self::Entity>>, WorkFeedError> {
        let user = viewer.user.clone();
        if !receipt.notification_ids.is_empty() {
            self.notifications
                .set_done(user.clone(), receipt.notification_ids.clone(), false)
                .await?;
        }
        let keys = receipt.item_keys();
        self.signals
            .items_changed(user, keys.iter().map(|key| key.entity().clone()).collect())
            .await;
        self.recompute(viewer, scope, keys).await
    }
}
