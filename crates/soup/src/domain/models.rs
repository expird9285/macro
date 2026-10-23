pub mod grouping;

mod exclusions;

#[cfg(test)]
mod test;

use call::domain::models::GetCallRecordsRequest;
use channels::domain::models::{GetChannelsRequest, GetThreadReplyRowsRequest};
use entity_access::domain::models::{EntityAccessReceipt, MemberTeamRole};
use filter_ast::Expr;
use foreign_entity::domain::{models::SourceId, ports::ForeignEntityListQuery};
use frecency::domain::models::{AggregateFrecency, FrecencyQueryErr};
use github_pull_requests::domain::models::GithubPullRequestError;
use item_filters::{
    EntityFilters,
    ast::{
        EntityFilterAst, ExpandErr, LiteralTree,
        call::CallLiteral,
        github_pull_request::GithubPullRequestLiteral,
        properties::{
            PropertiesLiteral, properties_filter_can_apply_to,
            properties_filter_matches_propertyless,
        },
    },
};
use macro_user_id::user_id::MacroUserIdStr;
use model_entity::{Entity, EntityType};
use models_grouping::GroupingConfig;
use models_pagination::{
    Cursor, CursorWithValAndFilter, Frecency, NotifiedAt, Query, SimpleSortMethod, TouchedByMe,
};
use models_soup::SoupProperty;
use models_soup::item::SoupItem;
use non_empty::IsEmpty;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Direction the merged Soup page is ordered in.
///
/// Deliberately Soup's own type rather than one borrowed from the paginator:
/// `Paginator` already exposes `sort_asc`/`sort_desc`, so this only has to name
/// the choice, and doing it here keeps the shared pagination crate untouched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SoupSortDirection {
    /// Smallest sort value first — oldest, or soonest for a future timestamp.
    Asc,
    /// Largest sort value first.
    #[default]
    Desc,
}

/// Controls whether soup items should include expanded nested data.
#[derive(Debug, Clone, Copy)]
pub enum SoupType {
    /// Return fully expanded soup items.
    Expanded,
    /// Return compact soup items without expanded nested data.
    UnExpanded,
}

/// the parameters required for a [SimpleSortMethod]
#[derive(Debug, Clone)]
pub struct SimpleSortRequest<'a> {
    /// the limit of the number of items to return
    pub(crate) limit: u16,
    /// the [Query] the client passes (if any)
    pub(crate) cursor: SimpleSortQuery,
    /// the id of the user
    pub(crate) user_id: MacroUserIdStr<'a>,
}

/// Parameters for grouped soup queries.
#[derive(Debug)]
pub struct GroupedSortRequest<'a> {
    /// the limit of the number of items to return
    pub limit: u16,
    /// the cursor/query
    pub cursor: Query<Uuid, SimpleSortMethod, EntityFilterAst>,
    /// the id of the user
    pub user_id: MacroUserIdStr<'a>,
    /// grouping configuration
    pub grouping: GroupingConfig,
}

#[derive(Debug, Clone)]
pub(crate) enum SimpleSortQuery {
    /// we dont have anything to filter out
    NoFilter(Query<Uuid, SimpleSortMethod, ()>),
    /// we filter out items that DO have a [Frecency] record
    FilterFrecency(Query<Uuid, SimpleSortMethod, Frecency>),
    /// we filter out items based on the input [EntityFilterAst]
    ItemsFilter(Query<Uuid, SimpleSortMethod, EntityFilterAst>),
    /// we filter out items based on the input [EntityFilterAst] IN ADDITION to ANY items that DO have a [Frecency] score
    ItemsAndFrecencyFilter(Query<Uuid, SimpleSortMethod, (Frecency, EntityFilterAst)>),
}

impl SimpleSortQuery {
    pub(crate) fn from_entity_cursor(
        cursor: Query<Uuid, SimpleSortMethod, Option<EntityFilterAst>>,
    ) -> Self {
        match cursor {
            Query::Sort(s, Some(f)) => SimpleSortQuery::ItemsFilter(Query::Sort(s, f)),
            Query::Sort(s, None) => SimpleSortQuery::NoFilter(Query::Sort(s, ())),
            Query::Cursor(Cursor {
                id,
                limit,
                val,
                filter: Some(filter),
            }) => SimpleSortQuery::ItemsFilter(Query::Cursor(Cursor {
                id,
                limit,
                val,
                filter,
            })),
            Query::Cursor(Cursor {
                id,
                limit,
                val,
                filter: None,
            }) => SimpleSortQuery::NoFilter(Query::Cursor(Cursor {
                id,
                limit,
                val,
                filter: (),
            })),
        }
    }
}

impl SimpleSortQuery {
    pub(crate) fn sort_method(&self) -> &SimpleSortMethod {
        match self {
            SimpleSortQuery::NoFilter(query) => query.sort_method(),
            SimpleSortQuery::FilterFrecency(query) => query.sort_method(),
            SimpleSortQuery::ItemsFilter(query) => query.sort_method(),
            SimpleSortQuery::ItemsAndFrecencyFilter(query) => query.sort_method(),
        }
    }
}

/// Parameters for fetching soup items by an explicit ordered entity list.
#[derive(Debug, Clone)]
pub struct AdvancedSortParams<'a> {
    /// Entities to fetch and preserve in the requested order.
    pub entities: &'a [Entity<'a>],
    /// User whose accessible soup items should be fetched.
    pub user_id: MacroUserIdStr<'a>,
}

/// Cursor or initial-sort query for a soup request.
#[derive(Debug)]
pub enum SoupQuery<T> {
    /// Query sorted by a simple timestamp-based method.
    Simple(SimpleQueryInner<T>),
    /// Query sorted by frecency.
    Frecency(FrecencyQueryInner<T>),
    /// Query filtered and sorted by the user's own latest mutation per
    /// entity (the activity log's touched-by-me surface).
    Touched(TouchedQueryInner<T>),
    /// Query filtered and sorted by the user's latest notification per
    /// entity (the inbox's notified-at surface).
    Notified(NotifiedQueryInner<T>),
}

impl<T> SoupQuery<T> {
    pub(crate) fn map<F, U>(self, f: F) -> SoupQuery<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            SoupQuery::Simple(SimpleQueryInner(i)) => {
                SoupQuery::Simple(SimpleQueryInner(i.map_filter(f)))
            }
            SoupQuery::Frecency(FrecencyQueryInner(i)) => {
                SoupQuery::Frecency(FrecencyQueryInner(i.map_filter(f)))
            }
            SoupQuery::Touched(TouchedQueryInner(i)) => {
                SoupQuery::Touched(TouchedQueryInner(i.map_filter(f)))
            }
            SoupQuery::Notified(NotifiedQueryInner(i)) => {
                SoupQuery::Notified(NotifiedQueryInner(i.map_filter(f)))
            }
        }
    }
}

/// the inner private type for [SoupQuery::Simple]
#[derive(Debug)]
pub struct SimpleQueryInner<T>(pub(crate) Query<Uuid, SimpleSortMethod, T>);

/// the inner private type for [SoupQuery::Frecency]
#[derive(Debug)]
pub struct FrecencyQueryInner<T>(pub(crate) Query<Uuid, Frecency, T>);

/// the inner private type for [SoupQuery::Touched]
///
/// The id is a [String] rather than a [Uuid]: touched cursors key into the
/// activity log's TEXT `entity_id` column, and the raw string must survive
/// the cursor round-trip byte-for-byte (see
/// [`TouchedPagePosition::entity_id`]).
#[derive(Debug)]
pub struct TouchedQueryInner<T>(pub(crate) Query<String, TouchedByMe, T>);

/// the inner private type for [SoupQuery::Notified]
///
/// The id is a [String] for the same reason as [`TouchedQueryInner`]: the
/// keyset compares it byte-for-byte against `notification.event_item_id`,
/// an unconstrained TEXT column.
#[derive(Debug)]
pub struct NotifiedQueryInner<T>(pub(crate) Query<String, NotifiedAt, T>);

impl<T> SoupQuery<T> {
    /// create a new instance of a [SimpleSortMethod] with [T] this is used to
    /// construct the initial page request. To paginate an existing cursor see [Self::new_cursor_simple]
    pub fn new_sort_simple(method: SimpleSortMethod, filters: T) -> Self {
        SoupQuery::Simple(SimpleQueryInner(models_pagination::Query::Sort(
            method, filters,
        )))
    }

    /// create a new instance of a [Frecency] with [T] this is used to
    /// construct the initial page request. To paginate an existing cursor see [Self::new_cursor_frecency]
    pub fn new_sort_frecency(method: Frecency, filters: T) -> Self {
        SoupQuery::Frecency(FrecencyQueryInner(models_pagination::Query::Sort(
            method, filters,
        )))
    }

    /// create a new instance of a [SimpleSortMethod] with an existing cursor on [T].
    /// This is used to continue paginating on an existing cursor.
    /// To create a new initial page see [Self::new_sort_simple]
    pub fn new_cursor_simple(cursor: CursorWithValAndFilter<Uuid, SimpleSortMethod, T>) -> Self {
        SoupQuery::Simple(SimpleQueryInner(models_pagination::Query::Cursor(cursor)))
    }

    /// create a new instance of a [Frecency] with an existing cursor on [T].
    /// This is used to continue paginating on an existing cursor.
    /// To create a new initial page see [Self::new_sort_simple]
    pub fn new_cursor_frecency(cursor: CursorWithValAndFilter<Uuid, Frecency, T>) -> Self {
        SoupQuery::Frecency(FrecencyQueryInner(models_pagination::Query::Cursor(cursor)))
    }

    /// create a new instance of a [TouchedByMe] query with [T]. This is used
    /// to construct the initial page request. To paginate an existing cursor
    /// see [Self::new_cursor_touched]
    pub fn new_sort_touched(filters: T) -> Self {
        SoupQuery::Touched(TouchedQueryInner(models_pagination::Query::Sort(
            TouchedByMe,
            filters,
        )))
    }

    /// create a new instance of a [TouchedByMe] query with an existing cursor
    /// on [T]. This is used to continue paginating on an existing cursor.
    /// To create a new initial page see [Self::new_sort_touched]
    pub fn new_cursor_touched(cursor: CursorWithValAndFilter<String, TouchedByMe, T>) -> Self {
        SoupQuery::Touched(TouchedQueryInner(models_pagination::Query::Cursor(cursor)))
    }

    /// create a new instance of a [NotifiedAt] query with [T]. This is used
    /// to construct the initial page request. To paginate an existing cursor
    /// see [Self::new_cursor_notified]
    pub fn new_sort_notified(filters: T) -> Self {
        SoupQuery::Notified(NotifiedQueryInner(models_pagination::Query::Sort(
            NotifiedAt, filters,
        )))
    }

    /// create a new instance of a [NotifiedAt] query with an existing cursor
    /// on [T]. This is used to continue paginating on an existing cursor.
    /// To create a new initial page see [Self::new_sort_notified]
    pub fn new_cursor_notified(cursor: CursorWithValAndFilter<String, NotifiedAt, T>) -> Self {
        SoupQuery::Notified(NotifiedQueryInner(models_pagination::Query::Cursor(cursor)))
    }

    /// Returns the filter payload embedded in this query.
    pub fn filter(&self) -> &T {
        match self {
            SoupQuery::Simple(SimpleQueryInner(query)) => query.filter(),
            SoupQuery::Frecency(FrecencyQueryInner(query)) => query.filter(),
            SoupQuery::Touched(TouchedQueryInner(query)) => query.filter(),
            SoupQuery::Notified(NotifiedQueryInner(query)) => query.filter(),
        }
    }
}

impl SoupQuery<EntityFilters> {
    /// Converts high-level entity filters into an AST-backed soup query.
    pub fn into_ast(self) -> Result<SoupQuery<Option<EntityFilterAst>>, ExpandErr> {
        match self {
            SoupQuery::Simple(SimpleQueryInner(query)) => Ok(SoupQuery::Simple(SimpleQueryInner(
                query.try_map_filter(EntityFilterAst::new_from_filters)?,
            ))),
            SoupQuery::Frecency(FrecencyQueryInner(query)) => Ok(SoupQuery::Frecency(
                FrecencyQueryInner(query.try_map_filter(EntityFilterAst::new_from_filters)?),
            )),
            SoupQuery::Touched(TouchedQueryInner(query)) => Ok(SoupQuery::Touched(
                TouchedQueryInner(query.try_map_filter(EntityFilterAst::new_from_filters)?),
            )),
            SoupQuery::Notified(NotifiedQueryInner(query)) => Ok(SoupQuery::Notified(
                NotifiedQueryInner(query.try_map_filter(EntityFilterAst::new_from_filters)?),
            )),
        }
    }
}

/// Request to fetch a page of soup items for a user.
#[derive(Debug)]
pub struct SoupRequest<T> {
    /// Whether to return expanded or unexpanded soup items.
    pub soup_type: SoupType,
    /// Maximum number of items to return.
    pub limit: u16,
    /// Initial query or pagination cursor to execute.
    pub cursor: SoupQuery<T>,
    /// Direction the merged page is ordered in. Defaults to descending, which
    /// is the default feed ordering.
    ///
    /// One choice per request: the paginator sorts the whole merged page, so
    /// this cannot differ per entity type within a single feed. It is a
    /// sibling of the cursor rather than part of it because clients re-send
    /// the request params on every page, which keeps the cursor format alone.
    ///
    /// Only the [`SoupQuery::Simple`] branch honours this. Frecency pages come
    /// back ordered by relevance score and never have the merged sort applied,
    /// so there is no ascending frecency order to produce — the REST adapter
    /// rejects that combination rather than letting it read as supported, and
    /// the GraphQL adapter cannot express frecency at all.
    pub sort_direction: SoupSortDirection,
    /// User whose soup should be queried.
    pub user: MacroUserIdStr<'static>,
}

/// trait which defines a type which can be fallibly converted into a SoupRequest with ast
pub trait IntoSoupReqAst {
    /// perform the conversion
    fn into_ast(self) -> Result<SoupRequest<Option<EntityFilterAst>>, ExpandErr>;
}

impl IntoSoupReqAst for SoupRequest<EntityFilters> {
    fn into_ast(self) -> Result<SoupRequest<Option<EntityFilterAst>>, ExpandErr> {
        let SoupRequest {
            soup_type,
            limit,
            cursor,
            sort_direction,
            user,
        } = self;

        Ok(SoupRequest {
            soup_type,
            limit,
            cursor: cursor.into_ast()?,
            sort_direction,
            user,
        })
    }
}

impl IntoSoupReqAst for SoupRequest<EntityFilterAst> {
    fn into_ast(self) -> Result<SoupRequest<Option<EntityFilterAst>>, ExpandErr> {
        let SoupRequest {
            soup_type,
            limit,
            cursor,
            sort_direction,
            user,
        } = self;

        Ok(SoupRequest {
            soup_type,
            limit,
            cursor: cursor.map(|f| if f.is_empty() { None } else { Some(f) }),
            sort_direction,
            user,
        })
    }
}

impl<T> SoupRequest<T> {
    /// returns a reference to the filters to pass to the paginator
    /// this is used to prevent passing back the full ast on each server request
    pub(crate) fn filters(&self) -> &T {
        self.cursor.filter()
    }
}

impl SoupRequest<Option<EntityFilterAst>> {
    /// The request's entity filter ast, regardless of cursor position.
    pub(crate) fn entity_ast(&self) -> Option<&EntityFilterAst> {
        match &self.cursor {
            SoupQuery::Simple(SimpleQueryInner(Query::Sort(_, f))) => f.as_ref(),
            SoupQuery::Simple(SimpleQueryInner(Query::Cursor(CursorWithValAndFilter {
                filter,
                ..
            }))) => filter.as_ref(),
            SoupQuery::Frecency(_) => None,
            SoupQuery::Touched(TouchedQueryInner(query)) => query.filter().as_ref(),
            SoupQuery::Notified(NotifiedQueryInner(query)) => query.filter().as_ref(),
        }
    }

    /// True when an active properties filter can never match an entity with
    /// no property rows. Property-less sub-requests (channels, channel
    /// threads, calls, foreign entities) are skipped so pages
    /// only contain rows the filter applies to.
    fn properties_filter_blocks_propertyless(&self) -> bool {
        self.entity_ast()
            .and_then(|a| a.properties_filter.as_deref())
            .is_some_and(|p| !properties_filter_matches_propertyless(p))
    }

    pub(crate) fn build_call_request(&self) -> Option<GetCallRecordsRequest> {
        if exclusions::call(self.entity_ast()) {
            return None;
        }
        // A def-less tag filter (entity_type None on every literal) applies to
        // calls, which carry tags; it is folded into the call filter below and
        // rendered as a `properties.values` EXISTS by the call query. Any other
        // active properties filter can't match a call, so the leg is skipped by
        // the propertyless check.
        let properties_filter = self
            .entity_ast()
            .and_then(|a| a.properties_filter.as_deref())
            .filter(|p| properties_filter_can_apply_to(p, &[]));
        if properties_filter.is_none() && self.properties_filter_blocks_propertyless() {
            return None;
        }
        Some(GetCallRecordsRequest {
            user_id: self.user.clone(),
            limit: self.limit as u32,
            query: match &self.cursor {
                SoupQuery::Simple(SimpleQueryInner(Query::Sort(t, f))) => Some(Query::Sort(
                    *t,
                    with_call_properties_filter(
                        f.as_ref().and_then(|f| f.call_filter.clone()),
                        properties_filter,
                    ),
                )),
                SoupQuery::Simple(SimpleQueryInner(Query::Cursor(CursorWithValAndFilter {
                    id,
                    limit,
                    val,
                    filter,
                }))) => Some(Query::Cursor(CursorWithValAndFilter {
                    id: *id,
                    limit: *limit,
                    val: val.clone(),
                    filter: with_call_properties_filter(
                        filter.as_ref().and_then(|f| f.call_filter.clone()),
                        properties_filter,
                    ),
                })),
                // query by frecency not yet implemented for call records
                SoupQuery::Frecency(_) => None,
                // call records carry no activity of their own (calls land on
                // the channel), so the touched feed never includes them
                SoupQuery::Touched(_) => None,
                // call notifications are not rolled into the notified feed
                SoupQuery::Notified(_) => None,
            }?,
        })
    }

    pub(crate) fn build_comms_request(&self) -> Option<GetChannelsRequest> {
        if self.properties_filter_blocks_propertyless() || exclusions::channel(self.entity_ast()) {
            return None;
        }
        Some(GetChannelsRequest {
            macro_id: self.user.clone(),
            limit: Some(self.limit as u32),
            include_frecency: false,
            query: match &self.cursor {
                SoupQuery::Simple(SimpleQueryInner(Query::Sort(t, f))) => Some(Query::Sort(
                    *t,
                    f.as_ref().and_then(|f| f.channel_filter.clone()),
                )),
                SoupQuery::Simple(SimpleQueryInner(Query::Cursor(CursorWithValAndFilter {
                    id,
                    limit,
                    val,
                    filter,
                }))) => Some(Query::Cursor(CursorWithValAndFilter {
                    id: *id,
                    limit: *limit,
                    val: val.clone(),
                    filter: filter.as_ref().and_then(|f| f.channel_filter.clone()),
                })),
                // query by frecency not yet implemented for channels
                SoupQuery::Frecency(_) => None,
                // touched-by-me hydrates channels by id, not via this leg
                SoupQuery::Touched(_) => None,
                // notified-at hydrates its channel candidates through this
                // leg with the request's own tree; the handler ANDs the ids in
                SoupQuery::Notified(NotifiedQueryInner(query)) => Some(Query::Sort(
                    SimpleSortMethod::UpdatedAt,
                    query
                        .filter()
                        .as_ref()
                        .and_then(|f| f.channel_filter.clone()),
                )),
            }?,
        })
    }

    pub(crate) fn build_comms_thread_request(&self) -> Option<GetThreadReplyRowsRequest> {
        if self.properties_filter_blocks_propertyless()
            || exclusions::channel_thread(self.entity_ast())
        {
            return None;
        }
        let query = match &self.cursor {
            SoupQuery::Simple(SimpleQueryInner(Query::Sort(t, f))) => Some(Query::Sort(
                *t,
                f.as_ref().and_then(|f| f.channel_thread_filter.clone()),
            )),
            SoupQuery::Simple(SimpleQueryInner(Query::Cursor(CursorWithValAndFilter {
                id,
                limit,
                val,
                filter,
            }))) => Some(Query::Cursor(CursorWithValAndFilter {
                id: *id,
                limit: *limit,
                val: val.clone(),
                filter: filter
                    .as_ref()
                    .and_then(|f| f.channel_thread_filter.clone()),
            })),
            // query by frecency not yet implemented for channel threads
            SoupQuery::Frecency(_) => None,
            // channel-thread rows carry no activity (messages attribute to
            // the channel), so the touched feed never includes them
            SoupQuery::Touched(_) => None,
            // notified-at hydrates thread-scoped channel notifications through
            // this leg with the request's own tree; the handler ANDs the
            // thread ids in
            SoupQuery::Notified(NotifiedQueryInner(query)) => Some(Query::Sort(
                SimpleSortMethod::UpdatedAt,
                query
                    .filter()
                    .as_ref()
                    .and_then(|f| f.channel_thread_filter.clone()),
            )),
        }?;

        Some(GetThreadReplyRowsRequest {
            macro_id: self.user.clone(),
            limit: Some(self.limit as u32),
            query,
        })
    }

    pub(crate) fn build_foreign_entity_query(&self) -> Option<ForeignEntityListQuery> {
        if self.properties_filter_blocks_propertyless()
            || exclusions::foreign_entity(self.entity_ast())
        {
            return None;
        }
        match &self.cursor {
            SoupQuery::Simple(SimpleQueryInner(Query::Sort(t, f))) => Some(Query::Sort(
                *t,
                f.as_ref()
                    .and_then(|filter| filter.foreign_entity_filter.clone()),
            )),
            SoupQuery::Simple(SimpleQueryInner(Query::Cursor(CursorWithValAndFilter {
                id,
                limit,
                val,
                filter,
            }))) => Some(Query::Cursor(CursorWithValAndFilter {
                id: *id,
                limit: *limit,
                val: val.clone(),
                filter: filter
                    .as_ref()
                    .and_then(|filter| filter.foreign_entity_filter.clone()),
            })),
            // query by frecency is not implemented for foreign entities
            SoupQuery::Frecency(_) => None,
            // foreign entities record no activity, so the touched feed never
            // includes them
            SoupQuery::Touched(_) => None,
            // notified-at hydrates its foreign-entity candidates through this
            // leg with the request's own tree; the handler ANDs the ids in
            SoupQuery::Notified(NotifiedQueryInner(query)) => Some(Query::Sort(
                SimpleSortMethod::UpdatedAt,
                query
                    .filter()
                    .as_ref()
                    .and_then(|filter| filter.foreign_entity_filter.clone()),
            )),
        }
    }

    /// The request's GitHub pull request filter, which narrows its foreign entity leg.
    pub(crate) fn build_github_pull_request_filter(&self) -> LiteralTree<GithubPullRequestLiteral> {
        self.entity_ast()
            .and_then(|ast| ast.github_pull_request_filter.clone())
    }

    pub(crate) fn build_foreign_entity_source_ids(
        &self,
        team_receipt: Option<&EntityAccessReceipt<MemberTeamRole>>,
    ) -> Vec<SourceId> {
        let mut source_ids = vec![SourceId::user(self.user.as_ref())];

        if let Some(receipt) = team_receipt {
            source_ids.push(SourceId::new(receipt.entity().entity_id.clone(), "team"));
        }

        source_ids
    }
}

/// Keyset position for a touched-by-me page: the touch timestamp and entity
/// id of the previous page's last row.
///
/// The id stays the raw stored string (never round-tripped through [`Uuid`]):
/// the SQL keyset compares it byte-for-byte against the `entity_id` TEXT
/// column, so any canonicalization would shift the page boundary.
#[derive(Debug, Clone)]
pub struct TouchedPagePosition {
    /// The last entity's latest-mutation timestamp.
    pub occurred_at: chrono::DateTime<chrono::Utc>,
    /// The last entity's id — the global tiebreaker when two entities share
    /// a touch timestamp.
    pub entity_id: String,
}

/// Parameters for one page of touched-by-me candidates.
#[derive(Debug)]
pub struct TouchedSoupRequest<'a> {
    /// User whose touches are listed; also the access-check subject.
    pub user_id: MacroUserIdStr<'a>,
    /// Maximum entities to return.
    pub limit: u16,
    /// Resume after this position; `None` for the first page.
    pub after: Option<TouchedPagePosition>,
    /// Entity filters folded into the candidate query.
    pub filter: Option<&'a EntityFilterAst>,
}

/// One touched entity: what it is and when the user last mutated it.
#[derive(Debug, Clone)]
pub struct TouchedEntity {
    /// The touched entity.
    pub entity: Entity<'static>,
    /// The user's latest mutation of it.
    pub touched_at: chrono::DateTime<chrono::Utc>,
}

/// Keyset position for a notified-at page: the notification timestamp and
/// event item id of the previous page's last candidate.
///
/// Like [`TouchedPagePosition`], the id stays the raw stored string: the SQL
/// keyset compares it byte-for-byte against `notification.event_item_id`.
#[derive(Debug, Clone)]
pub struct NotifiedPagePosition {
    /// When the last candidate's latest notification was created for the user.
    pub notified_at: chrono::DateTime<chrono::Utc>,
    /// The last candidate's event item id — the tiebreaker when two
    /// candidates share a notification timestamp.
    pub entity_id: String,
}

/// Parameters for one page of notified-at candidates.
#[derive(Debug)]
pub struct NotifiedSoupRequest<'a> {
    /// User whose notifications are listed; also the access-check subject.
    pub user_id: MacroUserIdStr<'a>,
    /// Maximum candidates to return.
    pub limit: u16,
    /// Resume after this position; `None` for the first page.
    pub after: Option<NotifiedPagePosition>,
    /// Entity filters folded into the candidate query where soup owns the
    /// fold (documents, chats, projects, properties).
    pub filter: Option<&'a EntityFilterAst>,
    /// Sources (the caller and their team) whose stored foreign entities the
    /// caller may see; gates foreign-entity candidates.
    pub foreign_entity_sources: &'a [SourceId],
    /// Entity types whose candidates the domain can hydrate for this request.
    /// Types outside this set are never returned, whatever the notifications
    /// table holds.
    pub hydratable: NotifiedHydratableTypes,
}

/// Which domain-hydrated entity types a notified-at request can surface.
///
/// The repository gates documents, chats and projects on
/// the filter tree itself; these are hydrated through other domains'
/// legs, so whether that leg is active for the request is decided by the
/// service and passed down here.
#[derive(Debug, Clone, Copy, Default)]
pub struct NotifiedHydratableTypes {
    /// The channel leg is active.
    pub channels: bool,
    /// The channel-thread leg is active. Thread-scoped channel notifications
    /// (mentions and replies, which name their thread root as the secondary
    /// event item) surface as thread rows, the way the client attributes them.
    pub channel_threads: bool,
    /// The foreign-entity leg is active.
    pub foreign_entities: bool,
}

/// One notified entity: what it is and when the user was last notified.
#[derive(Debug, Clone)]
pub struct NotifiedEntity {
    /// The entity the notification is about.
    pub entity: Entity<'static>,
    /// When the user's latest notification about it was created.
    pub notified_at: chrono::DateTime<chrono::Utc>,
}

/// Which reasons admit an entity to the work feed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkFeedSoupMode {
    /// Live notifications plus the viewer's own work.
    Work,
    /// Live notifications only.
    Attention,
}

/// Keyset position for a work feed page: the sort time and entity id of the
/// last candidate the previous page walked.
///
/// Like [`NotifiedPagePosition`], the id stays the raw stored string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkFeedPagePosition {
    /// The last candidate's sort time: the later of its attention and
    /// own-work timestamps.
    pub sort_at: chrono::DateTime<chrono::Utc>,
    /// The last candidate's entity id — the tiebreaker on equal sort times.
    pub entity_id: String,
}

/// Parameters for one page of work feed candidates.
#[derive(Debug)]
pub struct WorkFeedCandidateRequest<'a> {
    /// User whose feed is listed; also the access-check subject.
    pub user_id: MacroUserIdStr<'a>,
    /// Maximum candidates to return.
    pub limit: u16,
    /// Whether own work admits candidates.
    pub mode: WorkFeedSoupMode,
    /// Resume after this position; `None` for the first page.
    pub after: Option<WorkFeedPagePosition>,
    /// Entity types the caller asked for. Narrowed further by the filter
    /// and by which hydration legs are active.
    pub types: &'a [EntityType],
    /// Entity filters folded into the candidate gates where soup owns the
    /// fold, exactly as for the notified-at feed.
    pub filter: Option<&'a EntityFilterAst>,
    /// Sources whose stored foreign entities the caller may see.
    pub foreign_entity_sources: &'a [SourceId],
    /// Entity types whose candidates the domain can hydrate for this request.
    pub hydratable: NotifiedHydratableTypes,
    /// Restrict the feed to these candidate keys, for recomputing known items.
    pub only: Option<&'a [Entity<'static>]>,
}

/// One work feed candidate and the timestamps of its two reasons.
///
/// Each candidate carries the reason that places it in the feed: its
/// `sort_at` is the later of the two timestamps, and exactly one of the
/// candidate query's two streams emits it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkFeedCandidate {
    /// The candidate's entity; channel-thread candidates are keyed on their
    /// thread root as a `channel_message`, like the notified-at feed.
    pub entity: Entity<'static>,
    /// The newest pending (unseen or seen) notification keyed to the
    /// candidate. An own-work candidate omits attention older than the page's
    /// oldest own work, which the query does not read and which cannot move it.
    pub attention_at: Option<chrono::DateTime<chrono::Utc>>,
    /// The viewer's latest own (non-view) activity on the candidate. Always
    /// `None` in attention mode.
    pub touched_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Where the candidate sits in the feed.
    pub sort_at: chrono::DateTime<chrono::Utc>,
}

/// Request for one page of the hydrated work feed.
#[derive(Debug, Clone)]
pub struct WorkFeedSoupRequest {
    /// User whose feed is listed.
    pub user: MacroUserIdStr<'static>,
    /// Maximum items to return.
    pub limit: u16,
    /// Whether own work admits items.
    pub mode: WorkFeedSoupMode,
    /// Entity types to include.
    pub types: Vec<EntityType>,
    /// Membership filters, folded into the candidate gates and applied in
    /// full by each domain's hydration leg.
    pub filter: EntityFilterAst,
    /// Resume after this position; `None` for the first page.
    pub after: Option<WorkFeedPagePosition>,
    /// Restrict the feed to these candidate keys, for recomputing known items.
    pub only: Option<Vec<Entity<'static>>>,
}

/// One hydrated work feed item and its reason timestamps.
#[derive(Debug)]
pub struct WorkFeedSoupItem {
    /// The authorized Soup item with its server facts.
    pub hydration: SoupProjectionHydration,
    /// See [`WorkFeedCandidate::attention_at`].
    pub attention_at: Option<chrono::DateTime<chrono::Utc>>,
    /// See [`WorkFeedCandidate::touched_at`].
    pub touched_at: Option<chrono::DateTime<chrono::Utc>>,
    /// See [`WorkFeedCandidate::sort_at`].
    pub sort_at: chrono::DateTime<chrono::Utc>,
}

/// One page of the hydrated work feed.
#[derive(Debug)]
pub struct WorkFeedSoupPage {
    /// The page's items, newest first.
    pub items: Vec<WorkFeedSoupItem>,
    /// Where the next page starts; `None` once the feed is exhausted.
    pub next: Option<WorkFeedPagePosition>,
}

/// ANDs a properties filter (mapped to [CallLiteral::Property] conditions) into
/// the call filter tree, so the call query returns exactly the calls that also
/// satisfy the tag filter.
fn with_call_properties_filter(
    tree: Option<Arc<Expr<CallLiteral>>>,
    properties_filter: Option<&Expr<PropertiesLiteral>>,
) -> Option<Arc<Expr<CallLiteral>>> {
    let Some(propf) = properties_filter else {
        return tree;
    };
    let mapped = map_properties_to_call(propf);
    Some(Arc::new(match tree {
        Some(tree) => Expr::and((*tree).clone(), mapped),
        None => mapped,
    }))
}

fn map_properties_to_call(expr: &Expr<PropertiesLiteral>) -> Expr<CallLiteral> {
    match expr {
        Expr::And(a, b) => Expr::and(map_properties_to_call(a), map_properties_to_call(b)),
        Expr::Or(a, b) => Expr::or(map_properties_to_call(a), map_properties_to_call(b)),
        Expr::Not(a) => Expr::Not(Box::new(map_properties_to_call(a))),
        Expr::Literal(lit) => Expr::Literal(CallLiteral::Property(lit.clone())),
    }
}

/// Authoritative document facts that are unavailable as direct Soup fields.
///
/// This value is internal hydration state. It is deliberately separate from
/// public Soup entity models so relation-backed facts do not become business
/// fields on [`models_soup::document::SoupDocument`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoupDocumentServerFacts {
    /// Always `false`: documents are no longer linked to email attachments.
    /// The field stays so the cached projection wire format is unchanged.
    pub is_email_attachment: bool,
    /// Whether the document is important to the requesting viewer.
    ///
    /// Non-task documents are important. Tasks are important when the viewer
    /// appears in the authoritative Assignees system property.
    pub is_important: bool,
    /// Complete authoritative Status select-option IDs for a task.
    ///
    /// Non-task documents and tasks without a Status value carry an empty set.
    pub status_option_ids: Vec<uuid::Uuid>,
}

/// An authorized Soup item and any server-only facts read by the same
/// hydration operation.
///
/// `document_server_facts` is populated only when authoritative document
/// relation state was loaded. Projects, chats, unsupported entities, and
/// unenriched hydration paths leave it empty because they have no supplement
/// to emit.
#[derive(Debug, Clone)]
pub struct SoupProjectionHydration<Item = SoupItem<()>> {
    /// Authorized Soup item returned to the caller.
    pub item: Item,
    /// Relation-backed document facts unavailable in direct Soup fields.
    pub document_server_facts: Option<SoupDocumentServerFacts>,
}

impl<Item> SoupProjectionHydration<Item> {
    /// Map the hydrated item while preserving its authoritative server facts.
    pub fn map<F, Mapped>(self, map: F) -> SoupProjectionHydration<Mapped>
    where
        F: FnOnce(Item) -> Mapped,
    {
        SoupProjectionHydration {
            item: map(self.item),
            document_server_facts: self.document_server_facts,
        }
    }
}

/// A Soup result with both optional enrichments represented in one model.
///
/// Service methods decide which fields to populate. An unfetched enrichment
/// remains empty without changing the response model.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct EnrichedSoupItem {
    /// The Soup item with its property projection.
    pub item: SoupItem<SoupPropertiesField>,
    /// The aggregate frecency score, when requested and available.
    pub frecency_score: Option<AggregateFrecency>,
    /// The caller's latest own mutation of this entity, populated only when
    /// the page was ordered by `touched_by_me`. Clients sort and optimistic-
    /// reorder the touched feed on this value.
    pub touched_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When the caller was last notified about this entity, populated only
    /// when the page was ordered by `notified_at`. Clients sort and bucket
    /// the notified feed on this value.
    pub notified_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<SoupItem<SoupPropertiesField>> for EnrichedSoupItem {
    /// An item enriched with its properties alone, as
    /// [`SoupService::get_user_soup_with_properties`](super::ports::SoupService::get_user_soup_with_properties)
    /// answers it.
    fn from(item: SoupItem<SoupPropertiesField>) -> Self {
        Self {
            item,
            frecency_score: None,
            touched_at: None,
            notified_at: None,
        }
    }
}

/// A soup request with optional grouping configuration.
#[derive(Debug)]
pub struct GroupedSoupRequest<T> {
    /// Base soup request parameters
    pub base: SoupRequest<T>,
    /// Optional grouping configuration
    pub grouping: Option<GroupingConfig>,
}

/// Errors returned while building or executing soup queries.
#[derive(Debug, Error)]
pub enum SoupErr {
    /// Frecency query failed.
    #[error(transparent)]
    FrecencyErr(#[from] FrecencyQueryErr),
    /// Soup repository query failed.
    #[error(transparent)]
    SoupDbErr(#[from] anyhow::Error),
    /// Channel or communication lookup failed.
    #[error("A comms error has occured, see logs for more details")]
    CommsErr,
    /// Call record lookup failed.
    #[error("A call error has occurred, see logs for more details")]
    CallErr,
    /// A touched-by-me query carried a filter kind this mode cannot
    /// evaluate (the fold lives in another domain's query builder).
    /// Rejecting beats silently returning a feed with that type missing.
    #[error("sort_method=touched_by_me does not support {0} filters")]
    TouchedUnsupportedFilter(&'static str),
    /// A notified-at query carried a filter kind this mode cannot evaluate
    /// (the fold lives in another domain's query builder).
    #[error("sort_method=notified_at does not support {0} filters")]
    NotifiedUnsupportedFilter(&'static str),
    /// GitHub pull request listing failed.
    #[error(transparent)]
    GithubPullRequestErr(#[from] GithubPullRequestError),
    /// Entity filter AST expansion failed.
    #[error(transparent)]
    AstErr(#[from] ExpandErr),
}

/// Property fields that can be flattened into property-bearing Soup items.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "axum", derive(utoipa::ToSchema))]
pub struct SoupPropertiesField {
    /// Properties attached to the entity.
    pub properties: Vec<SoupProperty>,
}

/// A Soup item with entity properties attached.
pub type SoupItemWithProperties = SoupItem<SoupPropertiesField>;

impl SoupPropertiesField {
    /// Creates an extra field containing the supplied properties.
    pub fn new(properties: Vec<SoupProperty>) -> Self {
        Self { properties }
    }
}
