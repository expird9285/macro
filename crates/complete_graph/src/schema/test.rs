use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::http::{Request as HttpRequest, header};
use entity_access::domain::models::{
    AccessError, AccessLevel, BotAccessScope, BotId, CallChannelInfo, EntityAccessReceipt,
    EntityPermission, EntityType, RequiredPermission, TeamRole, UserTeamInfo,
};
use graphql_common::GraphqlRequestParts;
use macro_authorization::{
    INTERNAL_API_KEY_HEADER, INTERNAL_MACRO_USER_ID_HEADER, InternalIdentityClaims,
    MacroAuthorizationError, MacroAuthorizationService, MacroAuthorizationState,
};
use macro_user_id::{
    lowercased::Lowercase,
    user_id::{MacroUserId, MacroUserIdStr},
};
use model_entity::EntityType as ModelEntityType;
use model_owner::Owner;
use model_user::UserContext;
use models_pagination::{Paginated, PaginatedCursor, SimpleSortMethod};
use models_soup::{
    chat::SoupChat, document::SoupDocument, initiative::SoupInitiative, item::SoupItem,
    project::SoupProject,
};
use rootcause::Report;
use soup_realtime::domain::models::Patch;
use uuid::Uuid;

use super::*;

mod database_activity;
mod database_row;
mod form_activity;
mod initiative;
mod scheduled_actions;
mod soup_patches;
mod stale_client;
mod work_feed;

const VALID_USER_ID: &str = "macro|user@example.com";
const INTERNAL_USER_ID: &str = "macro|internal@example.com";
const VALID_INTERNAL_KEY: &str = "valid-internal-key";

#[derive(Clone)]
struct TestRealtimeSubscriptionService {
    receiver: Arc<Mutex<Option<tokio::sync::mpsc::Receiver<Patch<model_entity::Entity<'static>>>>>>,
    subscribed_user: Arc<Mutex<Option<MacroUserIdStr<'static>>>>,
}

impl SoupRealtimeSubscriptionService for TestRealtimeSubscriptionService {
    fn subscribe(
        &self,
        user_id: MacroUserIdStr<'static>,
    ) -> tokio::sync::mpsc::Receiver<Patch<model_entity::Entity<'static>>> {
        *self.subscribed_user.lock().expect("subscribed user lock") = Some(user_id);
        self.receiver
            .lock()
            .expect("subscription receiver lock")
            .take()
            .expect("test subscription is opened once")
    }
}

#[derive(Clone, Default)]
struct CountingSoupService {
    raw_calls: Arc<AtomicUsize>,
    raw_team_receipts: Arc<AtomicUsize>,
    return_empty_raw: bool,
    raw_response: Arc<Mutex<Option<Vec<SoupItem<()>>>>>,
    frecency_calls: Arc<AtomicUsize>,
    frecency_team_receipts: Arc<AtomicUsize>,
    grouped_calls: Arc<AtomicUsize>,
}

fn soup_document(id: Uuid) -> SoupItem<()> {
    grouped_document(id).map_extra(|_| ())
}

fn soup_agent_session(id: Uuid) -> SoupItem<()> {
    SoupItem::AgentSession(models_soup::agent_session::SoupAgentSession {
        id,
        name: "A session".to_string(),
        is_archived: false,
        owner_id: Owner::from_principal_str(VALID_USER_ID).unwrap(),
        bot_id: Uuid::from_u128(99),
        harness: "claude".to_string(),
        repo_url: None,
        repo_branch: None,
        pull_request_url: None,
        working_branch: None,
        pull_request_state: None,
        pull_request_id: None,
        turn_state: None,
        thread_id: None,
        status: "no_messages".to_string(),
        created_at: Default::default(),
        updated_at: Default::default(),
        viewed_at: None,
        extra: (),
    })
}

fn soup_initiative(id: Uuid) -> SoupItem<()> {
    SoupItem::Initiative(SoupInitiative {
        id,
        name: "Launch project".to_string(),
        owner_id: Owner::from_principal_str(VALID_USER_ID).unwrap(),
        created_at: Default::default(),
        updated_at: Default::default(),
        viewed_at: None,
        extra: (),
    })
}

fn soup_project(id: Uuid) -> SoupItem<()> {
    SoupItem::Project(SoupProject {
        id,
        name: format!("Project {id}"),
        owner_id: Owner::from_principal_str(VALID_USER_ID).unwrap(),
        parent_id: None,
        created_at: Default::default(),
        updated_at: Default::default(),
        viewed_at: None,
        deleted_at: None,
        extra: (),
    })
}

fn soup_chat(id: Uuid) -> SoupItem<()> {
    SoupItem::Chat(SoupChat {
        id,
        name: format!("Chat {id}"),
        model: Some("openai/gpt-5.6".to_string()),
        owner_id: Owner::from_principal_str(VALID_USER_ID).unwrap(),
        project_id: None,
        is_persistent: true,
        created_at: Default::default(),
        updated_at: Default::default(),
        viewed_at: None,
        deleted_at: None,
        extra: (),
    })
}

fn grouped_document(id: Uuid) -> SoupItem<soup::domain::models::SoupPropertiesField> {
    SoupItem::Document(SoupDocument {
        id,
        document_version_id: 1,
        owner_id: Owner::from_principal_str(VALID_USER_ID).unwrap(),
        name: format!("Document {id}"),
        file_type: None,
        sha: None,
        project_id: None,
        branched_from_id: None,
        branched_from_version_id: None,
        document_family_id: None,
        created_at: Default::default(),
        updated_at: Default::default(),
        viewed_at: Default::default(),
        sub_type: None,
        deleted_at: None,
        extra: soup::domain::models::SoupPropertiesField::default(),
    })
}

fn test_soup_err() -> soup::domain::models::SoupErr {
    soup::domain::models::SoupErr::SoupDbErr(anyhow::anyhow!("counting Soup service"))
}

impl CountingSoupService {
    fn set_raw_response(&self, items: Vec<SoupItem<()>>) {
        *self.raw_response.lock().expect("raw response lock") = Some(items);
    }
}

impl SoupService for CountingSoupService {
    async fn get_user_soup<T>(
        &self,
        _req: soup::domain::models::SoupRequest<T>,
        team_receipt: Option<EntityAccessReceipt<entity_access::domain::models::MemberTeamRole>>,
    ) -> Result<soup::domain::ports::SoupOutput<T>, soup::domain::models::SoupErr>
    where
        soup::domain::models::SoupRequest<T>: soup::domain::models::IntoSoupReqAst,
        T: Clone + serde::Serialize + Send,
    {
        self.raw_calls.fetch_add(1, Ordering::SeqCst);
        if team_receipt.is_some() {
            self.raw_team_receipts.fetch_add(1, Ordering::SeqCst);
        }
        let raw_response = self.raw_response.lock().expect("raw response lock").clone();
        if self.return_empty_raw || raw_response.is_some() {
            let page: PaginatedCursor<SoupItem<()>, String, SimpleSortMethod, T> =
                Paginated::from_parts(raw_response.unwrap_or_default(), None);
            return Ok(soup::domain::ports::SoupOutput::Simple(page));
        }
        Err(test_soup_err())
    }

    async fn get_user_soup_with_projection<T>(
        &self,
        req: soup::domain::models::SoupRequest<T>,
        team_receipt: Option<EntityAccessReceipt<entity_access::domain::models::MemberTeamRole>>,
    ) -> Result<
        soup::domain::ports::SoupOutput<T, soup::domain::models::SoupProjectionHydration>,
        soup::domain::models::SoupErr,
    >
    where
        soup::domain::models::SoupRequest<T>: soup::domain::models::IntoSoupReqAst,
        T: Clone + serde::Serialize + Send,
    {
        self.get_user_soup(req, team_receipt).await.map(|output| {
            output.map(|item| {
                let document_server_facts = matches!(&item, SoupItem::Document(_)).then_some(
                    soup::domain::models::SoupDocumentServerFacts {
                        is_email_attachment: false,
                        is_important: true,
                        status_option_ids: Vec::new(),
                    },
                );
                soup::domain::models::SoupProjectionHydration {
                    item,
                    document_server_facts,
                }
            })
        })
    }

    async fn get_user_soup_with_properties<T>(
        &self,
        _req: soup::domain::models::SoupRequest<T>,
        _team_receipt: Option<EntityAccessReceipt<entity_access::domain::models::MemberTeamRole>>,
    ) -> Result<
        soup::domain::ports::SoupOutput<T, soup::domain::models::EnrichedSoupItem>,
        soup::domain::models::SoupErr,
    >
    where
        soup::domain::models::SoupRequest<T>: soup::domain::models::IntoSoupReqAst,
        T: Clone + serde::Serialize + Send,
    {
        Err(test_soup_err())
    }

    async fn get_user_soup_with_frecency<T>(
        &self,
        _req: soup::domain::models::SoupRequest<T>,
        team_receipt: Option<EntityAccessReceipt<entity_access::domain::models::MemberTeamRole>>,
    ) -> Result<
        soup::domain::ports::SoupOutput<T, soup::domain::models::EnrichedSoupItem>,
        soup::domain::models::SoupErr,
    >
    where
        soup::domain::models::SoupRequest<T>: soup::domain::models::IntoSoupReqAst,
        T: Clone + serde::Serialize + Send,
    {
        self.frecency_calls.fetch_add(1, Ordering::SeqCst);
        if team_receipt.is_some() {
            self.frecency_team_receipts.fetch_add(1, Ordering::SeqCst);
        }
        Err(test_soup_err())
    }

    async fn get_user_soup_with_properties_and_frecency<T>(
        &self,
        _req: soup::domain::models::SoupRequest<T>,
        _team_receipt: Option<EntityAccessReceipt<entity_access::domain::models::MemberTeamRole>>,
    ) -> Result<
        soup::domain::ports::SoupOutput<T, soup::domain::models::EnrichedSoupItem>,
        soup::domain::models::SoupErr,
    >
    where
        soup::domain::models::SoupRequest<T>: soup::domain::models::IntoSoupReqAst,
        T: Clone + serde::Serialize + Send,
    {
        Err(test_soup_err())
    }

    async fn get_user_soup_grouped(
        &self,
        _req: soup::domain::models::GroupedSortRequest<'_>,
    ) -> Result<
        impl Iterator<
            Item = soup::domain::models::grouping::ItemGroupingInfo<
                soup::domain::models::SoupPropertiesField,
            >,
        > + Send,
        soup::domain::models::SoupErr,
    > {
        self.grouped_calls.fetch_add(1, Ordering::SeqCst);
        Ok(vec![
            soup::domain::models::grouping::ItemGroupingInfo {
                key: "document".to_string(),
                total_group_count: 3,
                index_in_group: 1,
                item: grouped_document(Uuid::from_u128(1)),
            },
            soup::domain::models::grouping::ItemGroupingInfo {
                key: "document".to_string(),
                total_group_count: 3,
                index_in_group: 2,
                item: grouped_document(Uuid::from_u128(2)),
            },
        ]
        .into_iter())
    }

    async fn caller_tag_sets<'a>(
        &self,
        _user_id: MacroUserIdStr<'a>,
    ) -> Result<
        Vec<models_properties::service::property_definition_with_options::PropertyDefinitionWithOptions>,
        soup::domain::models::SoupErr,
    >{
        Err(test_soup_err())
    }
}

#[derive(Clone, Default)]
struct RecordingActivityReader {
    edge_calls: Arc<Mutex<Vec<Vec<graphql_activity::ActivityEdgeKey>>>>,
    feed_calls: Arc<Mutex<Vec<(String, Option<(chrono::DateTime<chrono::Utc>, Uuid)>, u32)>>>,
    overview_calls: Arc<Mutex<Vec<(String, activity::ActivityWindow)>>>,
    records: Arc<Mutex<Vec<activity::ActivityRecord>>>,
}

impl RecordingActivityReader {
    fn set_records(&self, records: Vec<activity::ActivityRecord>) {
        *self.records.lock().expect("activity records lock") = records;
    }
}

impl graphql_activity::SoupActivityEdgeReader for RecordingActivityReader {
    async fn entity_activity(
        &self,
        keys: Vec<graphql_activity::ActivityEdgeKey>,
    ) -> HashMap<graphql_activity::ActivityEdgeKey, graphql_activity::ActivityEdgeLoad> {
        self.edge_calls
            .lock()
            .expect("activity edge calls lock")
            .push(keys.clone());
        let records = self.records.lock().expect("activity records lock").clone();
        keys.into_iter()
            .map(|key| {
                let matching = records
                    .iter()
                    .filter(|record| {
                        record.entity_type == key.entity.entity_type
                            && record.entity_id == key.entity.entity_id.as_ref()
                    })
                    .take(key.limit as usize)
                    .cloned()
                    .collect();
                (key, graphql_activity::ActivityEdgeLoad::Found(matching))
            })
            .collect()
    }
}

impl graphql_activity::ActivityFeedReader for RecordingActivityReader {
    async fn subject_feed(
        &self,
        subject_id: &str,
        cursor: Option<(chrono::DateTime<chrono::Utc>, Uuid)>,
        limit: std::num::NonZeroU32,
    ) -> Result<activity::ActivityFeedPage, graphql_activity::ActivityReadFailed> {
        let limit = limit.get();
        self.feed_calls
            .lock()
            .expect("activity feed calls lock")
            .push((subject_id.to_owned(), cursor, limit));
        // Emulate the repo: subject-scoped, newest-first keyset order,
        // strictly after the cursor position, at most `limit` rows, with
        // `next` set whenever more rows remain past the page.
        let records = self.records.lock().expect("activity records lock").clone();
        let mut page: Vec<activity::ActivityRecord> = records
            .into_iter()
            .filter(|record| record.subject_id == subject_id)
            .filter(|record| {
                cursor.is_none_or(|(occurred_at, id)| {
                    (record.occurred_at, record.id) < (occurred_at, id)
                })
            })
            .collect();
        let has_more = page.len() > limit as usize;
        page.truncate(limit as usize);
        let next = has_more
            .then(|| page.last().map(|record| (record.occurred_at, record.id)))
            .flatten();
        Ok(activity::ActivityFeedPage {
            records: page,
            next,
        })
    }

    async fn subject_overview(
        &self,
        subject_id: &str,
        window: activity::ActivityWindow,
    ) -> Result<activity::ActivityOverview, graphql_activity::ActivityReadFailed> {
        self.overview_calls
            .lock()
            .expect("activity overview calls lock")
            .push((subject_id.to_owned(), window.clone()));
        Ok(activity::ActivityOverview::new(
            window.clone(),
            vec![activity::DayCount {
                day: window.start,
                count: std::num::NonZeroU64::new(2).unwrap(),
            }],
            vec![activity::EntityRank {
                entity_type: ModelEntityType::Document,
                entity_id: "overview-doc".to_owned(),
                count: std::num::NonZeroU64::new(2).unwrap(),
            }],
        )
        .unwrap())
    }
}

/// Entity access service whose team lookups are counted and return a team.
#[derive(Clone, Default)]
struct CountingEntityAccessService {
    team_calls: Arc<AtomicUsize>,
}

impl EntityAccessService for CountingEntityAccessService {
    async fn generate_entity_access_receipt<T: RequiredPermission>(
        &self,
        _user_id: &MacroUserId<Lowercase<'_>>,
        _user_org_id: Option<i64>,
        entity_id: &str,
        entity_type: EntityType,
    ) -> Result<EntityAccessReceipt<T>, AccessError> {
        // The viewer views exactly one database, responds to one form (View),
        // edits another, and reaches nothing else. A level the grant does not
        // satisfy is refused as the real service refuses it.
        let grant = match (entity_type, entity_id) {
            (EntityType::Database, database_activity::VIEWABLE_DATABASE_ID)
            | (EntityType::Form, form_activity::RESPONDED_FORM_ID) => Some(AccessLevel::View),
            (EntityType::Form, form_activity::EDITABLE_FORM_ID) => Some(AccessLevel::Edit),
            (EntityType::Database | EntityType::Form, _) => None,
            _ => return Err(AccessError::internal("test access failure")),
        };
        if let Some(access_level) = grant {
            return EntityAccessReceipt::try_new_authenticated_user(
                MacroUserIdStr::parse_from_str(VALID_USER_ID).unwrap(),
                entity_access::domain::models::Entity {
                    entity_id: entity_id.to_owned(),
                    entity_type,
                },
                EntityPermission::AccessLevel { access_level },
            );
        }
        if matches!(entity_type, EntityType::Database | EntityType::Form) {
            return Err(AccessError::Unauthorized);
        }
        Err(AccessError::internal("test access failure"))
    }

    async fn generate_bot_entity_access_receipt<T: RequiredPermission>(
        &self,
        _bot_id: BotId,
        _scope: BotAccessScope,
        _entity_id: &str,
        _entity_type: EntityType,
    ) -> Result<EntityAccessReceipt<T>, AccessError> {
        Err(AccessError::internal("test access failure"))
    }

    async fn get_access_level(
        &self,
        _user_id: Option<&MacroUserId<Lowercase<'_>>>,
        _entity_id: &str,
        _entity_type: EntityType,
    ) -> Result<Option<AccessLevel>, AccessError> {
        Err(AccessError::internal("test access failure"))
    }

    async fn check_access(
        &self,
        _user_id: Option<&MacroUserId<Lowercase<'_>>>,
        _entity_id: &str,
        _entity_type: EntityType,
        _required_level: AccessLevel,
    ) -> Result<AccessLevel, AccessError> {
        Err(AccessError::internal("test access failure"))
    }

    async fn check_public_access(
        &self,
        _entity_id: &str,
        _entity_type: EntityType,
        _required_level: AccessLevel,
    ) -> Result<AccessLevel, AccessError> {
        Err(AccessError::internal("test access failure"))
    }

    async fn get_entity_permission(
        &self,
        _user_id: Option<&MacroUserId<Lowercase<'_>>>,
        _entity_id: &str,
        _entity_type: EntityType,
        _user_org_id: Option<i64>,
    ) -> Result<EntityPermission, AccessError> {
        Err(AccessError::internal("test access failure"))
    }

    async fn get_users_by_entity(
        &self,
        _entity_id: &str,
        _entity_type: EntityType,
    ) -> Result<Vec<MacroUserIdStr<'static>>, AccessError> {
        Err(AccessError::internal("test access failure"))
    }

    async fn get_call_channel(
        &self,
        _call_id: &Uuid,
    ) -> Result<Option<CallChannelInfo>, AccessError> {
        Err(AccessError::internal("test access failure"))
    }

    async fn get_call_channel_by_channel_id(
        &self,
        _channel_id: &Uuid,
    ) -> Result<Option<CallChannelInfo>, AccessError> {
        Err(AccessError::internal("test access failure"))
    }

    async fn get_user_team(
        &self,
        _user_id: &MacroUserId<Lowercase<'_>>,
    ) -> Result<Option<UserTeamInfo>, AccessError> {
        self.team_calls.fetch_add(1, Ordering::SeqCst);
        Ok(Some(UserTeamInfo {
            team_id: Uuid::from_u128(42),
            role: TeamRole::Member,
        }))
    }
}

#[derive(Clone, Default)]
struct FakeAuthorizationService {
    authorization_calls: Arc<AtomicUsize>,
}

impl MacroAuthorizationService for FakeAuthorizationService {
    async fn authorize(&self, jwt: &str) -> Result<UserContext, Report<MacroAuthorizationError>> {
        self.authorization_calls.fetch_add(1, Ordering::SeqCst);

        match jwt {
            "valid" => Ok(user_context(VALID_USER_ID)),
            "expired" => Err(Report::new(MacroAuthorizationError::CredentialsExpired)),
            _ => Err(Report::new(MacroAuthorizationError::InvalidCredentials)),
        }
    }

    async fn authorize_internal(
        &self,
        provided_key: &str,
        claims: InternalIdentityClaims,
    ) -> Result<Option<UserContext>, Report<MacroAuthorizationError>> {
        self.authorization_calls.fetch_add(1, Ordering::SeqCst);

        if provided_key != VALID_INTERNAL_KEY {
            return Err(Report::new(MacroAuthorizationError::InvalidCredentials));
        }

        Ok(claims.user_id.map(|user_id| user_context(&user_id)))
    }
}

fn user_context(user_id: &str) -> UserContext {
    UserContext {
        user_id: user_id.to_owned(),
        fusion_user_id: "fusion-user-id".to_owned(),
        permissions: None,
        organization_id: None,
    }
}

#[derive(Clone)]
struct TestState {
    authorization: MacroAuthorizationState<FakeAuthorizationService>,
    entity_access: Arc<CountingEntityAccessService>,
}

impl FromRef<TestState> for MacroAuthorizationState<FakeAuthorizationService> {
    fn from_ref(state: &TestState) -> Self {
        state.authorization.clone()
    }
}

impl FromRef<TestState> for Arc<CountingEntityAccessService> {
    fn from_ref(state: &TestState) -> Self {
        state.entity_access.clone()
    }
}

struct TestHarness {
    schema: SoupSchema<
        CountingSoupService,
        NoOpSoupRealtimeSubscriptionService,
        NoopWebSocketNotificationSubscriptionService,
        graphql_activity::NoOpActivitySubscriptionService,
        CountingEntityAccessService,
        FakeAuthorizationService,
        TestState,
        NoOpEntityPropertyWriter,
        UnavailableEntityMutationService,
        graphql_favorite::NoOpFavoriteMutationService,
        NoOpChannelActivityMutationService,
        NoOpNotificationMutationService,
        NoOpSoupNotificationEdgeReader,
        NoOpEntityPropertyReader,
        graphql_favorite::NoOpEntityFavoriteEdgeReader,
        graphql_permission::NoOpEntityPermissionEdgeReader,
        RecordingActivityReader,
    >,
    state: TestState,
    soup_service: CountingSoupService,
    /// Answers single agent-session lookups, as the primary does in DSS.
    primary_soup_service: CountingSoupService,
    activity_reader: RecordingActivityReader,
    authorization_calls: Arc<AtomicUsize>,
    team_calls: Arc<AtomicUsize>,
    raw_soup_calls: Arc<AtomicUsize>,
    raw_soup_team_receipts: Arc<AtomicUsize>,
    frecency_soup_calls: Arc<AtomicUsize>,
    frecency_soup_team_receipts: Arc<AtomicUsize>,
    grouped_soup_calls: Arc<AtomicUsize>,
}

fn harness() -> TestHarness {
    let entity_access = CountingEntityAccessService::default();
    let authorization = FakeAuthorizationService::default();
    let soup = CountingSoupService::default();
    let activity_reader = RecordingActivityReader::default();
    let authorization_calls = Arc::clone(&authorization.authorization_calls);
    let team_calls = Arc::clone(&entity_access.team_calls);
    let raw_soup_calls = Arc::clone(&soup.raw_calls);
    let raw_soup_team_receipts = Arc::clone(&soup.raw_team_receipts);
    let frecency_soup_calls = Arc::clone(&soup.frecency_calls);
    let frecency_soup_team_receipts = Arc::clone(&soup.frecency_team_receipts);
    let grouped_soup_calls = Arc::clone(&soup.grouped_calls);
    TestHarness {
        schema: build_schema_with_service(soup.clone()),
        state: TestState {
            authorization: MacroAuthorizationState::new(Arc::new(authorization)),
            entity_access: Arc::new(entity_access),
        },
        soup_service: soup,
        primary_soup_service: CountingSoupService::default(),
        activity_reader,
        authorization_calls,
        team_calls,
        raw_soup_calls,
        raw_soup_team_receipts,
        frecency_soup_calls,
        frecency_soup_team_receipts,
        grouped_soup_calls,
    }
}

fn authenticated_parts() -> axum::http::request::Parts {
    bearer_parts("valid")
}

fn bearer_parts(token: &str) -> axum::http::request::Parts {
    let request = HttpRequest::builder()
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(())
        .unwrap();
    request.into_parts().0
}

fn internal_parts(user_id: &str) -> axum::http::request::Parts {
    let request = HttpRequest::builder()
        .header(INTERNAL_API_KEY_HEADER, VALID_INTERNAL_KEY)
        .header(INTERNAL_MACRO_USER_ID_HEADER, user_id)
        .body(())
        .unwrap();
    request.into_parts().0
}

impl TestHarness {
    async fn execute(&self, query: &str) -> async_graphql::Response {
        self.execute_with_parts(query, authenticated_parts()).await
    }

    async fn execute_with_parts(
        &self,
        query: &str,
        parts: axum::http::request::Parts,
    ) -> async_graphql::Response {
        self.schema.execute(self.request(query, parts)).await
    }

    fn request(&self, query: &str, parts: axum::http::request::Parts) -> async_graphql::Request {
        let user_id = MacroUserIdStr::parse_from_str(VALID_USER_ID).unwrap();
        async_graphql::Request::new(query)
            .data(GraphqlRequestParts::new(parts))
            .data(self.state.clone())
            .data(graphql_soup::SoupItemDataLoader::new(
                graphql_soup::SoupItemLoader::new(self.soup_service.clone()),
            ))
            .data(graphql_soup::AgentSessionEntityLoader(
                graphql_soup::soup_item_loader_with_team_access(
                    self.primary_soup_service.clone(),
                    self.state.entity_access.as_ref().clone(),
                ),
            ))
            .data(graphql_soup::PrimarySoupItemLoader(
                graphql_soup::soup_item_loader_with_team_access(
                    self.primary_soup_service.clone(),
                    self.state.entity_access.as_ref().clone(),
                ),
            ))
            .data(graphql_properties::entity_properties_loader(
                user_id,
                NoOpEntityPropertyReader,
            ))
            .data(NoOpEntityFavoriteEdgeReader)
            .data(graphql_soup::FoldersQuery::noop())
            .data(self.activity_reader.clone())
            .data(graphql_activity::entity_activity_loader(
                self.activity_reader.clone(),
            ))
    }
}

#[tokio::test]
async fn soup_updates_subscribes_as_the_authenticated_user() {
    use async_graphql::futures_util::{StreamExt as _, pin_mut};

    let user_id = MacroUserIdStr::parse_from_str(VALID_USER_ID).unwrap();
    let subscribed_user = Arc::new(Mutex::new(None));
    let (sender, receiver) = tokio::sync::mpsc::channel(2);
    let realtime = TestRealtimeSubscriptionService {
        receiver: Arc::new(Mutex::new(Some(receiver))),
        subscribed_user: Arc::clone(&subscribed_user),
    };
    let soup_service = CountingSoupService::default();
    let document_id = Uuid::from_u128(42);
    let deleted_document_id = Uuid::from_u128(43);
    soup_service.set_raw_response(vec![soup_document(document_id)]);
    let loader =
        graphql_soup::PrimarySoupItemLoader(graphql_soup::soup_item_loader(soup_service.clone()));
    let schema: SoupSchema<
        CountingSoupService,
        TestRealtimeSubscriptionService,
        NoopWebSocketNotificationSubscriptionService,
        graphql_activity::NoOpActivitySubscriptionService,
        NoOpEntityAccessService,
        SchemaOnlyAuthorizationService,
        SchemaOnlyState,
        NoOpEntityPropertyWriter,
        UnavailableEntityMutationService,
        NoOpFavoriteMutationService,
        NoOpChannelActivityMutationService,
        NoOpNotificationMutationService,
        NoOpSoupNotificationEdgeReader,
        NoOpEntityPropertyReader,
        NoOpEntityFavoriteEdgeReader,
        NoOpEntityPermissionEdgeReader,
        NoOpActivityReader,
    > = build_schema_with_services(
        soup_service,
        realtime,
        NoopWebSocketNotificationSubscriptionService,
        graphql_activity::NoOpActivitySubscriptionService,
    );
    let request = async_graphql::Request::new(
        "subscription { soupUpdates { __typename ... on SoupUpdated { item { id cacheProjection } } ... on GraphqlCacheDeletion { graphqlTypeName entityId } } }",
    )
    .data(user_id.clone())
    .data(loader);
    let responses = schema.execute_stream(request);
    pin_mut!(responses);

    sender
        .send(Patch::Updated(
            ModelEntityType::Document.with_entity_string(document_id.to_string()),
        ))
        .await
        .expect("subscription remains open");
    sender
        .send(Patch::Deleted(
            ModelEntityType::Document.with_entity_string(deleted_document_id.to_string()),
        ))
        .await
        .expect("subscription remains open");
    let mut updates = Vec::new();
    while updates.len() < 2 {
        let response = responses.next().await.expect("subscription response");
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let data = response.data.into_json().expect("response data is JSON");
        updates.extend(
            data["soupUpdates"]
                .as_array()
                .expect("soupUpdates is a buffered list")
                .iter()
                .cloned(),
        );
    }

    assert_eq!(updates.len(), 2);
    assert_eq!(updates[0]["__typename"], "SoupUpdated");
    assert_eq!(updates[0]["item"]["id"], document_id.to_string());
    assert!(updates[0]["item"]["cacheProjection"].is_string());
    assert_eq!(updates[1]["__typename"], "GraphqlCacheDeletion");
    assert_eq!(updates[1]["graphqlTypeName"], "GraphqlSoupDocument");
    assert_eq!(updates[1]["entityId"], deleted_document_id.to_string());
    assert_eq!(
        subscribed_user
            .lock()
            .expect("subscribed user lock")
            .as_ref(),
        Some(&user_id)
    );
}

#[tokio::test]
async fn agent_session_is_read_from_the_primary_not_the_list_replica() {
    let harness = harness();
    let session_id = Uuid::from_u128(7);
    // Just created: the replica behind Soup lists has not seen the session yet.
    harness.soup_service.set_raw_response(Vec::new());
    harness
        .primary_soup_service
        .set_raw_response(vec![soup_agent_session(session_id)]);

    let response = harness
        .execute(&format!(
            r#"{{ user {{ agentSession(sessionId: "{session_id}") {{ id }} }} }}"#
        ))
        .await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let data = response.data.into_json().unwrap();
    assert_eq!(data["user"]["agentSession"]["id"], session_id.to_string());
    assert_eq!(harness.raw_soup_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn soup_entity_is_read_from_the_primary_not_the_list_replica() {
    let harness = harness();
    let document_id = Uuid::from_u128(8);
    // Just created: the replica behind Soup lists has not replayed the task yet.
    harness.soup_service.set_raw_response(Vec::new());
    harness
        .primary_soup_service
        .set_raw_response(vec![soup_document(document_id)]);

    let response = harness
        .execute(&format!(
            r#"{{ user {{ soupEntity(input: {{entityType: DOCUMENT, entityId: "{document_id}"}}) {{ __typename id }} }} }}"#
        ))
        .await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let data = response.data.into_json().unwrap();
    assert_eq!(
        data["user"]["soupEntity"]["__typename"],
        "GraphqlSoupDocument"
    );
    assert_eq!(data["user"]["soupEntity"]["id"], document_id.to_string());
    assert_eq!(harness.raw_soup_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn soup_entity_is_null_when_the_viewer_cannot_see_it() {
    let harness = harness();
    harness.primary_soup_service.set_raw_response(Vec::new());

    let response = harness
        .execute(&format!(
            r#"{{ user {{ soupEntity(input: {{entityType: DOCUMENT, entityId: "{}"}}) {{ id }} }} }}"#,
            Uuid::from_u128(9)
        ))
        .await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let data = response.data.into_json().unwrap();
    assert!(data["user"]["soupEntity"].is_null());
}

#[tokio::test]
async fn agent_session_log_appended_streams_runs_for_an_accessible_session() {
    use agent_runtime_protocol::domain::schema::v0::{SystemEvent, ToServerMessage};
    use agent_session::domain::model::{
        AgentSessionId, AgentSessionLog, Message, StoredAgentSessionLog,
    };
    use agent_session_realtime::domain::ports::AgentSessionLogSubscriptionService;
    use async_graphql::futures_util::{StreamExt as _, pin_mut};

    struct TestLogSubscriptions {
        receiver: Arc<Mutex<Option<tokio::sync::mpsc::Receiver<Vec<StoredAgentSessionLog>>>>>,
        subscribed: Arc<Mutex<Option<AgentSessionId>>>,
    }

    impl AgentSessionLogSubscriptionService for TestLogSubscriptions {
        fn subscribe(
            &self,
            session_id: AgentSessionId,
        ) -> tokio::sync::mpsc::Receiver<Vec<StoredAgentSessionLog>> {
            *self.subscribed.lock().expect("subscribed lock") = Some(session_id);
            self.receiver
                .lock()
                .expect("receiver lock")
                .take()
                .expect("subscribed once")
        }
    }

    let user_id = MacroUserIdStr::parse_from_str(VALID_USER_ID).unwrap();
    let session_id = Uuid::from_u128(7);
    let other_session_id = Uuid::from_u128(8);
    let (sender, receiver) = tokio::sync::mpsc::channel(2);
    let subscribed = Arc::new(Mutex::new(None));
    let subscriptions = crate::agent_session_log_subscriptions(TestLogSubscriptions {
        receiver: Arc::new(Mutex::new(Some(receiver))),
        subscribed: Arc::clone(&subscribed),
    });
    let soup_service = CountingSoupService::default();
    soup_service.set_raw_response(vec![soup_agent_session(session_id)]);
    let loader = graphql_soup::AgentSessionEntityLoader(graphql_soup::soup_item_loader(
        soup_service.clone(),
    ));
    let schema = build_schema_with_service::<
        CountingSoupService,
        NoOpEntityAccessService,
        SchemaOnlyAuthorizationService,
        SchemaOnlyState,
        NoOpEntityPropertyWriter,
        UnavailableEntityMutationService,
        NoOpFavoriteMutationService,
        NoOpChannelActivityMutationService,
        NoOpNotificationMutationService,
        NoOpSoupNotificationEdgeReader,
        NoOpEntityPropertyReader,
        NoOpEntityFavoriteEdgeReader,
        NoOpEntityPermissionEdgeReader,
        NoOpActivityReader,
    >(soup_service);
    let query = format!(
        "subscription {{ agentSessionLogAppended(sessionId: \"{session_id}\") {{ id userId direction content }} }}"
    );

    // A session the viewer cannot see cannot be followed.
    let refused = schema
        .execute_stream(
            async_graphql::Request::new(format!(
                "subscription {{ agentSessionLogAppended(sessionId: \"{other_session_id}\") {{ id }} }}"
            ))
            .data(user_id.clone())
            .data(loader.clone())
            .data(subscriptions.clone()),
        )
        .next()
        .await
        .expect("one response");
    assert!(!refused.errors.is_empty());
    assert!(subscribed.lock().expect("subscribed lock").is_none());

    let responses = schema.execute_stream(
        async_graphql::Request::new(query)
            .data(user_id)
            .data(loader)
            .data(subscriptions),
    );
    pin_mut!(responses);
    sender
        .send(vec![StoredAgentSessionLog {
            id: Uuid::from_u128(1),
            created_at: Default::default(),
            entry: AgentSessionLog {
                agent_session_id: AgentSessionId::new_from_uuid(session_id),
                user_id: None,
                content: Message::ToServer(ToServerMessage::Event {
                    event: SystemEvent::AcpReady,
                }),
                logged_at: None,
            },
        }])
        .await
        .expect("subscription remains open");
    drop(sender);

    let response = responses.next().await.expect("a run");
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let data = response.data.into_json().expect("response data is JSON");
    let run = data["agentSessionLogAppended"]
        .as_array()
        .expect("a run is a list");
    assert_eq!(run.len(), 1);
    assert_eq!(run[0]["id"], Uuid::from_u128(1).to_string());
    assert_eq!(run[0]["direction"], "to_server");
    assert!(run[0]["content"].is_object());
    assert_eq!(
        *subscribed.lock().expect("subscribed lock"),
        Some(AgentSessionId::new_from_uuid(session_id))
    );
    // The fan-out closing means rows were missed: the stream ends with an error.
    let closed = responses.next().await.expect("a closing response");
    assert!(!closed.errors.is_empty());
    assert!(responses.next().await.is_none());
}

#[tokio::test]
async fn activity_updates_subscribes_as_the_authenticated_user() {
    use activity::{
        ActivitySubscription, ActivitySubscriptionExit, ActivitySubscriptionService,
        ActivitySubscriptionUpdate,
    };
    use async_graphql::futures_util::{StreamExt as _, pin_mut};

    let user_id = MacroUserIdStr::parse_from_str(VALID_USER_ID).unwrap();
    let subscribed_user: Arc<Mutex<Option<MacroUserIdStr<'static>>>> = Arc::default();

    struct TestActivitySubscriptionService {
        subscribed_user: Arc<Mutex<Option<MacroUserIdStr<'static>>>>,
        subscription: Mutex<Option<ActivitySubscription>>,
    }

    impl ActivitySubscriptionService for TestActivitySubscriptionService {
        fn subscribe(&self, user_id: MacroUserIdStr<'static>) -> ActivitySubscription {
            *self.subscribed_user.lock().expect("subscribed user lock") = Some(user_id);
            self.subscription
                .lock()
                .expect("subscription lock")
                .take()
                .expect("subscription opened once")
        }
    }

    let (sender, receiver) = tokio::sync::mpsc::channel(2);
    let (exit_sender, exit_receiver) = tokio::sync::oneshot::channel();
    exit_sender
        .send(ActivitySubscriptionExit::Closed)
        .expect("exit receiver remains open");
    let service = TestActivitySubscriptionService {
        subscribed_user: Arc::clone(&subscribed_user),
        subscription: Mutex::new(Some(ActivitySubscription::from_parts(
            receiver,
            exit_receiver,
        ))),
    };

    let schema: SoupSchema<
        CountingSoupService,
        NoOpSoupRealtimeSubscriptionService,
        NoopWebSocketNotificationSubscriptionService,
        TestActivitySubscriptionService,
        NoOpEntityAccessService,
        SchemaOnlyAuthorizationService,
        SchemaOnlyState,
        NoOpEntityPropertyWriter,
        UnavailableEntityMutationService,
        NoOpFavoriteMutationService,
        NoOpChannelActivityMutationService,
        NoOpNotificationMutationService,
        NoOpSoupNotificationEdgeReader,
        NoOpEntityPropertyReader,
        NoOpEntityFavoriteEdgeReader,
        NoOpEntityPermissionEdgeReader,
        NoOpActivityReader,
    > = build_schema_with_services(
        CountingSoupService::default(),
        NoOpSoupRealtimeSubscriptionService,
        NoopWebSocketNotificationSubscriptionService,
        service,
    );

    let request = async_graphql::Request::new(
        "subscription { activityUpdates { __typename ... on GraphqlActivityEvent { id entityType entityId } } }",
    )
    .data(user_id.clone());
    let responses = schema.execute_stream(request);
    pin_mut!(responses);

    let record = {
        let activity = activity::Activity::common(
            Uuid::from_u128(7),
            0,
            activity::Actor::new_from_user(user_id.clone()),
            None,
            ModelEntityType::Document,
            "doc-1",
            activity::CommonAction::Edited,
            chrono::Utc::now(),
        );
        activity::ActivityRecord {
            id: activity.id,
            actor: activity.actor.clone(),
            subject_id: activity.subject_id.clone(),
            entity_type: activity.entity_type,
            entity_id: activity.entity_id.clone(),
            action: activity::RecordedAction::Known(activity::Action::Edited),
            occurred_at: activity.occurred_at,
        }
    };
    let record_id = record.id;
    sender
        .send(ActivitySubscriptionUpdate::Updated(Arc::new(record)))
        .await
        .expect("subscription remains open");

    let response = responses.next().await.expect("subscription response");
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let data = response.data.into_json().expect("response data is JSON");
    assert_eq!(data["activityUpdates"]["id"], record_id.to_string());
    assert_eq!(data["activityUpdates"]["entityType"], "DOCUMENT");
    assert_eq!(data["activityUpdates"]["entityId"], "doc-1");
    assert_eq!(
        subscribed_user
            .lock()
            .expect("subscribed user lock")
            .as_ref(),
        Some(&user_id)
    );
}

#[tokio::test]
async fn user_id_resolves_without_touching_services() {
    let harness = harness();

    let response = harness.execute("{ user { id } }").await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    assert_eq!(
        response.data.to_string(),
        r#"{user: {id: "macro|user@example.com"}}"#
    );
    assert_eq!(harness.authorization_calls.load(Ordering::SeqCst), 1);
    assert_eq!(harness.team_calls.load(Ordering::SeqCst), 0);
    assert_eq!(harness.raw_soup_calls.load(Ordering::SeqCst), 0);
    assert_eq!(harness.frecency_soup_calls.load(Ordering::SeqCst), 0);
    assert_eq!(harness.grouped_soup_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn favorites_are_nested_under_the_authenticated_user() {
    let harness = harness();

    let response = harness
        .execute("{ user { favorites { entityType entityId sortOrder } } }")
        .await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    assert_eq!(response.data.to_string(), "{user: {favorites: []}}");
    assert_eq!(harness.authorization_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn folders_are_nested_under_the_authenticated_user() {
    let harness = harness();

    let response = harness
        .execute("{ user { folders { id name parentId ownerId } } }")
        .await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    assert_eq!(response.data.to_string(), "{user: {folders: []}}");
    assert_eq!(harness.authorization_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn internal_authorization_uses_the_acting_user_claim() {
    let harness = harness();

    let response = harness
        .execute_with_parts("{ user { id } }", internal_parts(INTERNAL_USER_ID))
        .await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    assert_eq!(
        response.data.to_string(),
        r#"{user: {id: "macro|internal@example.com"}}"#
    );
    assert_eq!(harness.authorization_calls.load(Ordering::SeqCst), 1);
    assert_eq!(harness.team_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn soup_passes_team_receipt_to_raw_path() {
    let harness = harness();

    let _response = harness
        .execute("{ user { soup(input: {initial: {}}) { nextCursor } } }")
        .await;

    assert_eq!(harness.team_calls.load(Ordering::SeqCst), 1);
    assert_eq!(harness.raw_soup_calls.load(Ordering::SeqCst), 1);
    assert_eq!(harness.raw_soup_team_receipts.load(Ordering::SeqCst), 1);
    assert_eq!(harness.frecency_soup_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        harness.frecency_soup_team_receipts.load(Ordering::SeqCst),
        0
    );
}

#[tokio::test]
async fn flat_soup_emits_server_fact_supplements_for_documents() {
    let harness = harness();
    let document_id = Uuid::from_u128(88);
    let project_id = Uuid::from_u128(90);
    let chat_id = Uuid::from_u128(91);
    harness.soup_service.set_raw_response(vec![
        soup_document(document_id),
        soup_project(project_id),
        soup_chat(chat_id),
    ]);

    let response = harness
        .execute(
            "{ user { soup(input: {initial: {}}) { items { __typename id cacheProjection } } } }",
        )
        .await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let data = response.data.into_json().unwrap();
    let items = data["user"]["soup"]["items"].as_array().unwrap();
    let document = items
        .iter()
        .find(|item| item["__typename"] == "GraphqlSoupDocument")
        .unwrap();
    assert_eq!(document["id"], document_id.to_string());
    assert!(document["cacheProjection"].is_string());
    for (typename, id) in [
        ("GraphqlSoupProject", project_id),
        ("GraphqlSoupChat", chat_id),
    ] {
        let item = items
            .iter()
            .find(|item| item["__typename"] == typename)
            .unwrap();
        assert_eq!(item["id"], id.to_string());
        assert!(item["cacheProjection"].is_null());
    }
}

#[tokio::test]
async fn soup_input_rejects_initial_and_continuation_together() {
    let harness = harness();

    let response = harness
        .execute(
            r#"{ user { soup(input: {initial: {}, continuation: {cursor: "invalid"}}) { nextCursor } } }"#,
        )
        .await;

    assert!(!response.errors.is_empty());
    assert_eq!(harness.raw_soup_calls.load(Ordering::SeqCst), 0);
    assert_eq!(harness.frecency_soup_calls.load(Ordering::SeqCst), 0);
    assert_eq!(harness.team_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn soup_returns_the_saved_chat_model() {
    let harness = harness();
    harness
        .soup_service
        .set_raw_response(vec![soup_chat(Uuid::from_u128(91))]);
    let response = harness.execute(
        "{ user { soup(input: {initial: {}}) { items { ... on GraphqlSoupChat { model } } } } }"
    ).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let data = response.data.into_json().unwrap();
    assert_eq!(data["user"]["soup"]["items"][0]["model"], "openai/gpt-5.6");
}

#[tokio::test]
async fn soup_passes_team_receipt_to_frecency_enriched_path() {
    let harness = harness();

    let _response = harness
        .execute("{ user { soup(input: {initial: {}}) { items { frecencyScore } } } }")
        .await;

    assert_eq!(harness.raw_soup_calls.load(Ordering::SeqCst), 0);
    assert_eq!(harness.raw_soup_team_receipts.load(Ordering::SeqCst), 0);
    assert_eq!(harness.frecency_soup_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        harness.frecency_soup_team_receipts.load(Ordering::SeqCst),
        1
    );
    assert_eq!(harness.team_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn group_soup_nests_items_in_bins_and_reuses_cached_authorization() {
    let harness = harness();

    let response = harness
        .execute(
            "{ user { id groupSoup(input: {initial: {groupBy: {field: ENTITY_TYPE}}}) { bins { key totalCount nextCursor items { id entityType } } } } }",
        )
        .await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let data = response.data.into_json().unwrap();
    let bin = &data["user"]["groupSoup"]["bins"][0];
    assert_eq!(bin["key"], "document");
    assert_eq!(bin["totalCount"], 3);
    assert_eq!(harness.authorization_calls.load(Ordering::SeqCst), 1);
    let cursor = bin["nextCursor"].as_str().unwrap();
    assert_eq!(bin["items"].as_array().unwrap().len(), 2);

    let continuation = format!(
        "{{ user {{ groupSoup(input: {{continuation: {{groupBy: {{field: ENTITY_TYPE}}, groupKey: \"document\", cursor: \"{cursor}\"}}}}) {{ bins {{ key }} }} }} }}"
    );
    let response = harness.execute(&continuation).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);

    assert_eq!(harness.authorization_calls.load(Ordering::SeqCst), 2);
    assert_eq!(harness.grouped_soup_calls.load(Ordering::SeqCst), 2);
    assert_eq!(harness.team_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn unauthenticated_request_fails_at_the_resolver() {
    let harness = harness();
    let parts = HttpRequest::new(()).into_parts().0;

    let response = harness.execute_with_parts("{ user { id } }", parts).await;

    assert_eq!(response.errors.len(), 1);
    assert_eq!(response.errors[0].message, "authentication required");
    assert_eq!(harness.authorization_calls.load(Ordering::SeqCst), 0);
    assert_eq!(harness.team_calls.load(Ordering::SeqCst), 0);
    assert_eq!(harness.raw_soup_calls.load(Ordering::SeqCst), 0);
    assert_eq!(harness.frecency_soup_calls.load(Ordering::SeqCst), 0);
    assert_eq!(harness.grouped_soup_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn invalid_credentials_return_the_safe_authorization_error() {
    let harness = harness();

    let response = harness
        .execute_with_parts("{ user { id } }", bearer_parts("invalid"))
        .await;

    assert_eq!(response.errors.len(), 1);
    assert_eq!(response.errors[0].message, "unauthorized");
    assert_eq!(harness.authorization_calls.load(Ordering::SeqCst), 1);
    assert_eq!(harness.team_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn expired_credentials_return_the_safe_authorization_error() {
    let harness = harness();

    let response = harness
        .execute_with_parts("{ user { id } }", bearer_parts("expired"))
        .await;

    assert_eq!(response.errors.len(), 1);
    assert_eq!(response.errors[0].message, "jwt expired");
    assert_eq!(harness.authorization_calls.load(Ordering::SeqCst), 1);
    assert_eq!(harness.team_calls.load(Ordering::SeqCst), 0);
}

fn activity_record(
    id: u128,
    entity_type: ModelEntityType,
    entity_id: &str,
    action: activity::RecordedAction,
    occurred_at_secs: i64,
) -> activity::ActivityRecord {
    activity::ActivityRecord {
        id: Uuid::from_u128(id),
        actor: activity::Actor::new_from_user(
            MacroUserIdStr::parse_from_str(VALID_USER_ID).unwrap(),
        ),
        subject_id: VALID_USER_ID.to_owned(),
        entity_type,
        entity_id: entity_id.to_owned(),
        action,
        occurred_at: chrono::DateTime::from_timestamp(occurred_at_secs, 0)
            .expect("valid timestamp"),
    }
}

#[tokio::test]
async fn entity_activity_is_lazy_and_batches_across_entities() {
    let harness = harness();
    let first_id = Uuid::from_u128(61);
    let second_id = Uuid::from_u128(62);
    harness
        .soup_service
        .set_raw_response(vec![soup_document(first_id), soup_document(second_id)]);
    harness.activity_reader.set_records(vec![activity_record(
        1,
        ModelEntityType::Document,
        &first_id.to_string(),
        activity::RecordedAction::Known(activity::Action::Edited),
        300,
    )]);

    let without_activity = harness
        .execute(
            r#"{ user { soup(input: {initial: {}}) { items { ... on GraphqlSoupDocument { id } } } } }"#,
        )
        .await;
    assert!(
        without_activity.errors.is_empty(),
        "{:?}",
        without_activity.errors
    );
    assert!(
        harness
            .activity_reader
            .edge_calls
            .lock()
            .expect("activity edge calls lock")
            .is_empty(),
        "unselected activity edge must not touch the reader"
    );

    let with_activity = harness
        .execute(
            r#"{ user { soup(input: {initial: {}}) { items { ... on GraphqlSoupDocument { id activity { occurredAt action { __typename } } } } } } }"#,
        )
        .await;
    assert!(
        with_activity.errors.is_empty(),
        "{:?}",
        with_activity.errors
    );

    let calls = harness
        .activity_reader
        .edge_calls
        .lock()
        .expect("activity edge calls lock")
        .clone();
    assert_eq!(calls.len(), 1, "both entities load in one batched call");
    let mut requested: Vec<(String, u32)> = calls[0]
        .iter()
        .map(|key| (key.entity.entity_id.to_string(), key.limit))
        .collect();
    requested.sort();
    assert_eq!(
        requested,
        vec![(first_id.to_string(), 10), (second_id.to_string(), 10)]
    );

    let items = &with_activity.data.into_json().unwrap()["user"]["soup"]["items"];
    assert_eq!(
        items[0]["activity"][0]["action"]["__typename"],
        "GraphqlActivityEdited"
    );
    assert!(items[0]["activity"][0]["occurredAt"].as_str().is_some());
    assert_eq!(
        items[1]["activity"].as_array().map(Vec::len),
        Some(0),
        "entity with no activity resolves an empty timeline"
    );
}

#[tokio::test]
async fn activity_feed_pages_by_cursor_and_carries_unknown_actions() {
    let harness = harness();
    let doc = Uuid::from_u128(71).to_string();
    harness.activity_reader.set_records(vec![
        activity_record(
            3,
            ModelEntityType::Document,
            &doc,
            activity::RecordedAction::Unknown {
                tag: "transmogrified".to_owned(),
                payload: None,
            },
            300,
        ),
        activity_record(
            2,
            ModelEntityType::Document,
            &doc,
            activity::RecordedAction::Known(activity::Action::Edited),
            200,
        ),
        activity_record(
            1,
            ModelEntityType::Document,
            &doc,
            activity::RecordedAction::Known(activity::Action::Created),
            100,
        ),
    ]);

    let first_page = harness
        .execute(
            r#"{ user { activity(input: {limit: 2}) { items { entityId entityType action { __typename ... on GraphqlActivityUnknownAction { tag } } } nextCursor } } }"#,
        )
        .await;
    assert!(first_page.errors.is_empty(), "{:?}", first_page.errors);
    let data = first_page.data.into_json().unwrap();
    let page = &data["user"]["activity"];
    assert_eq!(page["items"].as_array().map(Vec::len), Some(2));
    assert_eq!(
        page["items"][0]["action"]["__typename"],
        "GraphqlActivityUnknownAction"
    );
    assert_eq!(page["items"][0]["action"]["tag"], "transmogrified");
    assert_eq!(page["items"][0]["entityId"], doc);
    assert_eq!(page["items"][0]["entityType"], "DOCUMENT");
    assert_eq!(
        page["items"][1]["action"]["__typename"],
        "GraphqlActivityEdited"
    );
    let cursor = page["nextCursor"].as_str().expect("more rows exist");

    let second_page = harness
        .execute(&format!(
            r#"{{ user {{ activity(input: {{limit: 2, cursor: "{cursor}"}}) {{ items {{ action {{ __typename }} }} nextCursor }} }} }}"#
        ))
        .await;
    assert!(second_page.errors.is_empty(), "{:?}", second_page.errors);
    let data = second_page.data.into_json().unwrap();
    let page = &data["user"]["activity"];
    assert_eq!(page["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        page["items"][0]["action"]["__typename"],
        "GraphqlActivityCreated"
    );
    assert!(page["nextCursor"].is_null());

    let feed_calls = harness
        .activity_reader
        .feed_calls
        .lock()
        .expect("activity feed calls lock")
        .clone();
    // The resolver binds the viewer's principal string as the subject and
    // passes the client limit through; the has-more probe lives in storage.
    assert_eq!(feed_calls.len(), 2);
    assert_eq!(feed_calls[0].0, VALID_USER_ID);
    assert_eq!(feed_calls[0].2, 2);
    let (cursor_at, cursor_id) = feed_calls[1].1.expect("second page carries the cursor");
    assert_eq!(cursor_at, chrono::DateTime::from_timestamp(200, 0).unwrap());
    assert_eq!(cursor_id, Uuid::from_u128(2));
}

#[tokio::test]
async fn activity_overview_uses_the_authenticated_subject_and_requested_zone() {
    let harness = harness();

    let response = harness
        .execute(
            r#"{ user { activityOverview(input: {timeZone: "America/Havana"}) { from to timeZone total days { date count } topEntities { entityType entityId count } } } }"#,
        )
        .await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let calls = harness
        .activity_reader
        .overview_calls
        .lock()
        .expect("activity overview calls lock")
        .clone();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, VALID_USER_ID);
    assert_eq!(calls[0].1.zone.name(), "America/Havana");

    let data = response.data.into_json().unwrap();
    let overview = &data["user"]["activityOverview"];
    assert_eq!(overview["from"], calls[0].1.start.to_string());
    assert_eq!(overview["to"], calls[0].1.end.to_string());
    assert_eq!(overview["timeZone"], "America/Havana");
    assert_eq!(overview["total"], 2);
    assert_eq!(overview["days"][0]["date"], calls[0].1.start.to_string());
    assert_eq!(overview["days"][0]["count"], 2);
    assert_eq!(overview["topEntities"][0]["entityType"], "DOCUMENT");
    assert_eq!(overview["topEntities"][0]["entityId"], "overview-doc");
    assert_eq!(overview["topEntities"][0]["count"], 2);
}

#[tokio::test]
async fn initiatives_are_returned_by_soup_with_shared_edges() {
    let harness = harness();
    let id = Uuid::from_u128(42);
    harness
        .soup_service
        .set_raw_response(vec![soup_initiative(id)]);
    let response = harness
        .execute(
            r#"{
        user { soup(input: {initial: {filters: {initiativeFilter: {literal: {include: true}}}}}) {
            items {
                __typename id entityType displayName
                metadata { ownerId ownerType createdAt updatedAt }
                properties { id }
            }
        } }
    }"#,
        )
        .await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let data = response.data.into_json().unwrap();
    let item = &data["user"]["soup"]["items"][0];
    assert_eq!(item["__typename"], "GraphqlSoupInitiative");
    assert_eq!(item["id"], id.to_string());
    assert_eq!(item["entityType"], "INITIATIVE");
    assert_eq!(item["displayName"], "Launch project");
    assert_eq!(item["metadata"]["ownerId"], VALID_USER_ID);
    assert!(item["properties"].is_array());
    assert_eq!(harness.raw_soup_calls.load(Ordering::SeqCst), 1);
}
