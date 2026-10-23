use axum::{
    Router,
    http::{Method, Request, StatusCode, header},
};
use chrono::{Duration, Utc};
use entity_access::domain::models::{BotAccessScope, BotId};
use http_body_util::BodyExt;
use item_filters::EntityFilters;
use macro_authorization::{
    INTERNAL_API_KEY_HEADER, INTERNAL_MACRO_USER_ID_HEADER, InternalIdentityClaims,
    MacroAuthorizationError, MacroAuthorizationService, MacroAuthorizationState,
};
use macro_user_id::user_id::MacroUserIdStr;
use model_user::UserContext;
use models_pagination::{
    CursorVal, CursorWithValAndFilter, Frecency, FrecencyValue, Identify, PaginateOn, Query,
    SimpleSortMethod, SortOn, TypeEraseCursor,
};
use rootcause::Report;
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;
use tower::util::ServiceExt;
use uuid::Uuid;

use item_filters::ast::EntityFilterAst;

use crate::{
    domain::{
        models::{
            EnrichedSoupItem, FrecencyQueryInner, GroupedSortRequest, IntoSoupReqAst,
            NotifiedQueryInner, SimpleQueryInner, SoupErr, SoupPropertiesField, SoupQuery,
            SoupRequest, SoupType, TouchedQueryInner, grouping::ItemGroupingInfo,
        },
        ports::{SoupOutput, SoupService},
    },
    inbound::axum_router::ApiEntityFilterAst,
    inbound::axum_router::{SoupRouterState, soup_router},
};

static CURSOR: &str = "eyJpZCI6ImUzNmM5MTJlLTU2M2MtNDIxZS1iMTAzLWE0YjAwY2ZmMzBlZSIsImxpbWl0IjoxMDAsInZhbCI6eyJzb3J0X3R5cGUiOiJ1cGRhdGVkX2F0IiwibGFzdF92YWwiOiIyMDI1LTExLTA3VDE5OjEyOjU5Ljc4MFoifSwiZmlsdGVyIjp7fX0=";
const USER_ID: &str = "macro|test@example.com";
const INTERNAL_USER_ID: &str = "macro|internal@example.com";
const VALID_INTERNAL_KEY: &str = "valid-internal-key";

#[derive(Debug)]
enum MockCursorKind {
    SimpleSort,
    SimpleCursor,
    FrecencySort,
    FrecencyCursor,
    TouchedSort,
    TouchedCursor,
    NotifiedSort,
    NotifiedCursor,
}

#[derive(Debug)]
struct MockSoupCall {
    user_id: String,
    soup_type: SoupType,
    cursor_kind: MockCursorKind,
    filter: serde_json::Value,
    expanded_filter: serde_json::Value,
}

#[derive(Clone)]
struct MockSoup {
    called: Arc<std::sync::Mutex<Vec<MockSoupCall>>>,
    succeeds: bool,
}

impl MockSoup {
    fn new() -> Self {
        Self {
            called: Arc::new(std::sync::Mutex::new(Vec::new())),
            succeeds: false,
        }
    }

    fn succeeding() -> Self {
        Self {
            succeeds: true,
            ..Self::new()
        }
    }
}

impl SoupService for MockSoup {
    async fn get_user_soup<T>(
        &self,
        _req: SoupRequest<T>,
        _team_receipt: Option<
            entity_access::domain::models::EntityAccessReceipt<
                entity_access::domain::models::MemberTeamRole,
            >,
        >,
    ) -> Result<SoupOutput<T>, SoupErr>
    where
        SoupRequest<T>: IntoSoupReqAst,
        T: Clone + Serialize + Send,
    {
        Err(SoupErr::SoupDbErr(anyhow::anyhow!("Not implemented")))
    }

    async fn get_user_soup_with_properties<T>(
        &self,
        _req: SoupRequest<T>,
        _team_receipt: Option<
            entity_access::domain::models::EntityAccessReceipt<
                entity_access::domain::models::MemberTeamRole,
            >,
        >,
    ) -> Result<SoupOutput<T, EnrichedSoupItem>, SoupErr>
    where
        SoupRequest<T>: IntoSoupReqAst,
        T: Clone + Serialize + Send,
    {
        Err(SoupErr::SoupDbErr(anyhow::anyhow!("Not implemented")))
    }

    async fn get_user_soup_with_frecency<T>(
        &self,
        _req: SoupRequest<T>,
        _team_receipt: Option<
            entity_access::domain::models::EntityAccessReceipt<
                entity_access::domain::models::MemberTeamRole,
            >,
        >,
    ) -> Result<SoupOutput<T, EnrichedSoupItem>, SoupErr>
    where
        SoupRequest<T>: IntoSoupReqAst,
        T: Clone + Serialize + Send,
    {
        Err(SoupErr::SoupDbErr(anyhow::anyhow!("Not implemented")))
    }

    async fn get_user_soup_with_properties_and_frecency<T>(
        &self,
        req: SoupRequest<T>,
        _team_receipt: Option<
            entity_access::domain::models::EntityAccessReceipt<
                entity_access::domain::models::MemberTeamRole,
            >,
        >,
    ) -> Result<SoupOutput<T, EnrichedSoupItem>, SoupErr>
    where
        SoupRequest<T>: IntoSoupReqAst,
        T: Clone + Serialize + Send,
    {
        let cursor_kind = match &req.cursor {
            SoupQuery::Simple(SimpleQueryInner(Query::Sort(..))) => MockCursorKind::SimpleSort,
            SoupQuery::Simple(SimpleQueryInner(Query::Cursor(..))) => MockCursorKind::SimpleCursor,
            SoupQuery::Frecency(FrecencyQueryInner(Query::Sort(..))) => {
                MockCursorKind::FrecencySort
            }
            SoupQuery::Frecency(FrecencyQueryInner(Query::Cursor(..))) => {
                MockCursorKind::FrecencyCursor
            }
            SoupQuery::Touched(TouchedQueryInner(Query::Sort(..))) => MockCursorKind::TouchedSort,
            SoupQuery::Touched(TouchedQueryInner(Query::Cursor(..))) => {
                MockCursorKind::TouchedCursor
            }
            SoupQuery::Notified(NotifiedQueryInner(Query::Sort(..))) => {
                MockCursorKind::NotifiedSort
            }
            SoupQuery::Notified(NotifiedQueryInner(Query::Cursor(..))) => {
                MockCursorKind::NotifiedCursor
            }
        };
        let user_id = req.user.to_string();
        let soup_type = req.soup_type;
        let filter = serde_json::to_value(req.cursor.filter()).unwrap();
        let expanded_filter = serde_json::to_value(req.into_ast()?.cursor.filter()).unwrap();
        let mut guard = self.called.lock().unwrap();
        guard.push(MockSoupCall {
            user_id,
            soup_type,
            cursor_kind,
            filter,
            expanded_filter,
        });
        drop(guard);

        if self.succeeds {
            Ok(SoupOutput::Simple(
                models_pagination::Paginated::from_parts(Vec::new(), None),
            ))
        } else {
            Err(SoupErr::SoupDbErr(anyhow::anyhow!("Not implemented")))
        }
    }

    async fn get_user_soup_grouped(
        &self,
        _req: GroupedSortRequest<'_>,
    ) -> Result<impl Iterator<Item = ItemGroupingInfo<SoupPropertiesField>> + Send, SoupErr> {
        Err::<std::vec::IntoIter<ItemGroupingInfo<SoupPropertiesField>>, _>(SoupErr::SoupDbErr(
            anyhow::anyhow!("Not implemented"),
        ))
    }

    async fn caller_tag_sets<'a>(
        &self,
        _user_id: MacroUserIdStr<'a>,
    ) -> Result<
        Vec<models_properties::service::property_definition_with_options::PropertyDefinitionWithOptions>,
        SoupErr,
    >{
        Err(SoupErr::SoupDbErr(anyhow::anyhow!("Not implemented")))
    }
}

#[derive(Clone)]
struct MockEntityAccess;

impl entity_access::domain::ports::EntityAccessService for MockEntityAccess {
    async fn generate_entity_access_receipt<
        T: entity_access::domain::models::RequiredPermission,
    >(
        &self,
        _user_id: &macro_user_id::user_id::MacroUserId<macro_user_id::lowercased::Lowercase<'_>>,
        _user_org_id: Option<i64>,
        _entity_id: &str,
        _entity_type: entity_access::domain::models::EntityType,
    ) -> Result<
        entity_access::domain::models::EntityAccessReceipt<T>,
        entity_access::domain::models::AccessError,
    > {
        unimplemented!()
    }

    async fn generate_bot_entity_access_receipt<
        T: entity_access::domain::models::RequiredPermission,
    >(
        &self,
        _bot_id: BotId,
        _scope: BotAccessScope,
        _entity_id: &str,
        _entity_type: entity_access::domain::models::EntityType,
    ) -> Result<
        entity_access::domain::models::EntityAccessReceipt<T>,
        entity_access::domain::models::AccessError,
    > {
        unimplemented!()
    }

    async fn get_access_level(
        &self,
        _user_id: Option<
            &macro_user_id::user_id::MacroUserId<macro_user_id::lowercased::Lowercase<'_>>,
        >,
        _entity_id: &str,
        _entity_type: entity_access::domain::models::EntityType,
    ) -> Result<
        Option<entity_access::domain::models::AccessLevel>,
        entity_access::domain::models::AccessError,
    > {
        unimplemented!()
    }

    async fn check_access(
        &self,
        _user_id: Option<
            &macro_user_id::user_id::MacroUserId<macro_user_id::lowercased::Lowercase<'_>>,
        >,
        _entity_id: &str,
        _entity_type: entity_access::domain::models::EntityType,
        _required_level: entity_access::domain::models::AccessLevel,
    ) -> Result<
        entity_access::domain::models::AccessLevel,
        entity_access::domain::models::AccessError,
    > {
        unimplemented!()
    }

    async fn check_public_access(
        &self,
        _entity_id: &str,
        _entity_type: entity_access::domain::models::EntityType,
        _required_level: entity_access::domain::models::AccessLevel,
    ) -> Result<
        entity_access::domain::models::AccessLevel,
        entity_access::domain::models::AccessError,
    > {
        unimplemented!()
    }

    async fn get_entity_permission(
        &self,
        _user_id: Option<
            &macro_user_id::user_id::MacroUserId<macro_user_id::lowercased::Lowercase<'_>>,
        >,
        _entity_id: &str,
        _entity_type: entity_access::domain::models::EntityType,
        _user_org_id: Option<i64>,
    ) -> Result<
        entity_access::domain::models::EntityPermission,
        entity_access::domain::models::AccessError,
    > {
        unimplemented!()
    }

    async fn get_crm_entity_permission_with_team(
        &self,
        _user_id: Option<
            &macro_user_id::user_id::MacroUserId<macro_user_id::lowercased::Lowercase<'_>>,
        >,
        _entity_id: &str,
        _entity_type: entity_access::domain::models::EntityType,
    ) -> Result<
        (
            entity_access::domain::models::EntityPermission,
            uuid::Uuid,
            entity_access::domain::models::TeamRole,
        ),
        entity_access::domain::models::AccessError,
    > {
        unimplemented!()
    }

    async fn get_users_by_entity(
        &self,
        _entity_id: &str,
        _entity_type: entity_access::domain::models::EntityType,
    ) -> Result<Vec<MacroUserIdStr<'static>>, entity_access::domain::models::AccessError> {
        unimplemented!()
    }

    async fn get_call_channel(
        &self,
        _call_id: &Uuid,
    ) -> Result<
        Option<entity_access::domain::models::CallChannelInfo>,
        entity_access::domain::models::AccessError,
    > {
        unimplemented!()
    }

    async fn get_call_channel_by_channel_id(
        &self,
        _channel_id: &Uuid,
    ) -> Result<
        Option<entity_access::domain::models::CallChannelInfo>,
        entity_access::domain::models::AccessError,
    > {
        unimplemented!()
    }

    async fn get_user_team(
        &self,
        _user_id: &macro_user_id::user_id::MacroUserId<macro_user_id::lowercased::Lowercase<'_>>,
    ) -> Result<
        Option<entity_access::domain::models::UserTeamInfo>,
        entity_access::domain::models::AccessError,
    > {
        Ok(None)
    }
}

#[derive(Clone)]
struct FakeAuthorizationService;

impl MacroAuthorizationService for FakeAuthorizationService {
    async fn authorize(&self, jwt: &str) -> Result<UserContext, Report<MacroAuthorizationError>> {
        match jwt {
            "valid" => Ok(user_context(USER_ID)),
            "expired" => Err(Report::new(MacroAuthorizationError::CredentialsExpired)),
            _ => Err(Report::new(MacroAuthorizationError::InvalidCredentials)),
        }
    }

    async fn authorize_internal(
        &self,
        provided_key: &str,
        claims: InternalIdentityClaims,
    ) -> Result<Option<UserContext>, Report<MacroAuthorizationError>> {
        if provided_key != VALID_INTERNAL_KEY {
            return Err(Report::new(MacroAuthorizationError::InvalidCredentials));
        }

        Ok(claims.user_id.map(|user_id| user_context(&user_id)))
    }
}

fn user_context(user_id: &str) -> UserContext {
    UserContext {
        user_id: user_id.to_string(),
        fusion_user_id: "fusion-user-id".to_string(),
        permissions: None,
        organization_id: None,
    }
}

type MockRouterState<T> = SoupRouterState<T, MockEntityAccess, FakeAuthorizationService>;

fn mock_router_state<T>(service: T) -> MockRouterState<T>
where
    T: SoupService,
{
    SoupRouterState::new(
        service,
        Arc::new(MockEntityAccess),
        MacroAuthorizationState::new(Arc::new(FakeAuthorizationService)),
    )
}

fn mock_router_with<T>(service: T) -> Router
where
    T: SoupService,
{
    soup_router(mock_router_state(service))
}

fn mock_router() -> Router {
    mock_router_with(MockSoup::new())
}

fn authenticated_request() -> axum::http::request::Builder {
    Request::builder().header(header::AUTHORIZATION, "Bearer valid")
}

async fn send_json(
    router: Router,
    request: Request<axum::body::Body>,
) -> (StatusCode, serde_json::Value) {
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap();
    (status, body)
}

#[tokio::test]
async fn it_should_deserialize_empty_filter() {
    let request = authenticated_request()
        .uri(format!("/soup?cursor={CURSOR}"))
        .body(axum::body::Body::empty())
        .unwrap();

    let (status, body) = send_json(mock_router(), request).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        body,
        json!({
            "message": "An internal server error has occurred"
        })
    );
}

#[tokio::test]
async fn no_credentials_returns_unauthorized_json() {
    let request = Request::get(format!("/soup?cursor={CURSOR}"))
        .body(axum::body::Body::empty())
        .unwrap();

    let (status, body) = send_json(mock_router(), request).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, json!({ "message": "unauthorized" }));
}

#[tokio::test]
async fn garbage_bearer_returns_unauthorized_json() {
    let request = Request::get(format!("/soup?cursor={CURSOR}"))
        .header(header::AUTHORIZATION, "Bearer garbage")
        .body(axum::body::Body::empty())
        .unwrap();

    let (status, body) = send_json(mock_router(), request).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, json!({ "message": "unauthorized" }));
}

#[tokio::test]
async fn internal_credentials_use_the_acting_user() {
    let soup = MockSoup::succeeding();
    let calls = soup.called.clone();
    let request = Request::get(format!("/soup?cursor={CURSOR}"))
        .header(INTERNAL_API_KEY_HEADER, VALID_INTERNAL_KEY)
        .header(INTERNAL_MACRO_USER_ID_HEADER, INTERNAL_USER_ID)
        .body(axum::body::Body::empty())
        .unwrap();

    let (status, body) = send_json(mock_router_with(soup), request).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "items": [], "next_cursor": null }));

    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].user_id, INTERNAL_USER_ID);
}

#[tokio::test]
async fn it_parses_file_assoc_filters() {
    let soup = MockSoup::new();
    let inner_counter = soup.called.clone();
    let router: Router = mock_router_with(soup);

    let request = authenticated_request()
        .uri("/soup")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&serde_json::json!({
                "document_filters": {
                    "file_types": ["assoc:other"]
                }
            }))
            .unwrap(),
        ))
        .unwrap();

    let _res = router.oneshot(request).await.unwrap();

    let guard = inner_counter.lock().unwrap();
    let arg = guard.first().unwrap();
    assert!(matches!(arg.soup_type, SoupType::Expanded));
    assert!(matches!(arg.cursor_kind, MockCursorKind::SimpleSort));
}

#[tokio::test]
async fn cursor_with_assoc_works() {
    let soup = MockSoup::new();
    let inner_counter = soup.called.clone();
    let router: Router = mock_router_with(soup.clone());

    let request = authenticated_request()
        .uri("/soup")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&serde_json::json!({
                "document_filters": {
                    "file_types": ["assoc:other"]
                }
            }))
            .unwrap(),
        ))
        .unwrap();

    let _res = router.oneshot(request).await.unwrap();

    let arg = {
        let mut guard = inner_counter.lock().unwrap();
        guard.pop().unwrap()
    };

    let filter: EntityFilters = serde_json::from_value(arg.filter).unwrap();

    #[derive(Serialize)]
    struct Data(usize, Uuid);

    impl Identify for Data {
        type Id = Uuid;

        fn id(&self) -> Uuid {
            self.1
        }
    }

    impl SortOn<Frecency> for Data {
        fn sort_on(_sort: Frecency) -> impl FnMut(&Self) -> models_pagination::CursorVal<Frecency> {
            |v| CursorVal {
                sort_type: Frecency,
                last_val: FrecencyValue::FrecencyScore(v.0 as f64),
            }
        }
    }

    // create arbitrary pagination data
    let res = (0..1000)
        .map(|x| Data(x, Uuid::new_v4()))
        .paginate_on(100, Frecency)
        .filter_on(filter)
        .into_page();

    let cursor = res.type_erase().next_cursor.unwrap();

    let request2 = authenticated_request()
        .uri(format!("/soup?cursor={cursor}"))
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&serde_json::json!({})).unwrap(),
        ))
        .unwrap();

    let router: Router = mock_router_with(soup);

    let _res = router.oneshot(request2).await.unwrap();
    let guard2 = inner_counter.lock().unwrap();
    let req = guard2.first().unwrap();
    assert!(matches!(req.cursor_kind, MockCursorKind::FrecencyCursor));
}

#[tokio::test]
async fn cursor_with_all_works() {
    let soup = MockSoup::new();
    let inner_counter = soup.called.clone();
    let router: Router = mock_router_with(soup.clone());

    let request = authenticated_request()
        .uri("/soup")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&serde_json::json!({
                "document_filters": {
                    "file_types": ["assoc:code", "assoc:other", "assoc:image", "md", "pdf", "canvas"]
                }
            }))
            .unwrap(),
        ))
        .unwrap();

    let _res = router.oneshot(request).await.unwrap();

    let arg = {
        let mut guard = inner_counter.lock().unwrap();
        guard.pop().unwrap()
    };

    let filter: EntityFilters = serde_json::from_value(arg.filter).unwrap();

    #[derive(Serialize)]
    struct Data(usize, Uuid);

    impl Identify for Data {
        type Id = Uuid;

        fn id(&self) -> Uuid {
            self.1
        }
    }

    impl SortOn<Frecency> for Data {
        fn sort_on(_sort: Frecency) -> impl FnMut(&Self) -> models_pagination::CursorVal<Frecency> {
            |v| CursorVal {
                sort_type: Frecency,
                last_val: FrecencyValue::FrecencyScore(v.0 as f64),
            }
        }
    }

    // create arbitrary pagination data
    let res = (0..1000)
        .map(|x| Data(x, Uuid::new_v4()))
        .paginate_on(100, Frecency)
        .filter_on(filter)
        .into_page();

    let cursor = res.type_erase().next_cursor.unwrap();

    let request2 = authenticated_request()
        .uri(format!("/soup?cursor={cursor}"))
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&serde_json::json!({})).unwrap(),
        ))
        .unwrap();

    let router: Router = mock_router_with(soup);

    let _res = router.oneshot(request2).await.unwrap();
    let guard2 = inner_counter.lock().unwrap();
    let req = guard2.first().unwrap();
    assert!(matches!(req.cursor_kind, MockCursorKind::FrecencyCursor));
}

#[tokio::test]
async fn it_parses_channel_filters() {
    let soup = MockSoup::new();
    let inner_counter = soup.called.clone();
    let router: Router = mock_router_with(soup.clone());

    let uuid1 = Uuid::new_v4();
    let uuid2 = Uuid::new_v4();
    let request = authenticated_request()
        .uri("/soup")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&serde_json::json!({
                "channel_filters": {
                    "channel_ids": [uuid1, uuid2]
                }
            }))
            .unwrap(),
        ))
        .unwrap();

    let _res = router.oneshot(request).await.unwrap();

    let arg = {
        let mut guard = inner_counter.lock().unwrap();
        guard.pop().unwrap()
    };

    let filter: EntityFilters = serde_json::from_value(arg.filter).unwrap();
    assert!(!filter.channel_filters.channel_ids.is_empty());
}

#[tokio::test]
async fn it_parses_notification_and_task_filters() {
    let soup = MockSoup::new();
    let inner_counter = soup.called.clone();
    let router: Router = mock_router_with(soup);

    let request = authenticated_request()
        .uri("/soup")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&serde_json::json!({
                "document_filters": {
                    "notification_filters": { "states": ["unseen"] },
                    "task_filters": { "include_cbm_atm_nc": true }
                },
                "chat_filters": {
                    "notification_filters": { "states": ["unseen"] }
                },
                "project_filters": {
                    "notification_filters": { "states": ["unseen"] }
                },
                "channel_filters": {
                    "notification_filters": { "states": ["unseen"] }
                }
            }))
            .unwrap(),
        ))
        .unwrap();

    let _res = router.oneshot(request).await.unwrap();

    let arg = {
        let mut guard = inner_counter.lock().unwrap();
        guard.pop().unwrap()
    };

    let filter: EntityFilters = serde_json::from_value(arg.filter).unwrap();
    assert_eq!(
        filter.document_filters.notification_filters.states,
        vec![item_filters::NotificationState::Unseen]
    );
    assert_eq!(
        filter.document_filters.task_filters.include_cbm_atm_nc,
        Some(true)
    );
    assert_eq!(
        filter.chat_filters.notification_filters.states,
        vec![item_filters::NotificationState::Unseen]
    );
    assert_eq!(
        filter.project_filters.notification_filters.states,
        vec![item_filters::NotificationState::Unseen]
    );
    assert_eq!(
        filter.channel_filters.notification_filters.states,
        vec![item_filters::NotificationState::Unseen]
    );
}

#[tokio::test]
async fn it_can_filter_chat_owners() {
    let json = r#"{
"channel_filters": {
"channel_ids": [
"00000000-0000-0000-0000-000000000000"
]
},
"document_filters": {
"document_ids": [
"00000000-0000-0000-0000-000000000000"
]
},
"project_filters": {
"project_ids": [
"00000000-0000-0000-0000-000000000000"
]
},
"chat_filters": {
"owners": [
"macro|rahul@macro.com"
]
},
"limit": 100,
"sort_method": "updated_at"
}"#;

    let soup = MockSoup::new();
    let inner_counter = soup.called.clone();
    let router: Router = mock_router_with(soup);

    let request = authenticated_request()
        .uri("/soup")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(json))
        .unwrap();

    let _res = router.oneshot(request).await.unwrap();

    let arg = {
        let mut guard = inner_counter.lock().unwrap();
        guard.pop().unwrap()
    };

    let filter: EntityFilters = serde_json::from_value(arg.filter).unwrap();
    assert_eq!(
        filter.chat_filters.owners,
        vec!["macro|rahul@macro.com".to_string()]
    );
}

#[tokio::test]
async fn ast_endpoint_expands_file_assoc_pdf() {
    let soup = MockSoup::new();
    let inner_counter = soup.called.clone();
    let router: Router = mock_router_with(soup);

    let request = authenticated_request()
        .uri("/soup/ast")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&json!({
                "df": { "l": { "fa": "assoc:pdf" } }
            }))
            .unwrap(),
        ))
        .unwrap();

    let _res = router.oneshot(request).await.unwrap();

    let arg = {
        let mut guard = inner_counter.lock().unwrap();
        guard
            .pop()
            .expect("SoupService::handle should have been called")
    };

    let filter: EntityFilterAst = serde_json::from_value(arg.expanded_filter).unwrap();
    let doc_tree = filter
        .document_filter
        .expect("document_filter should be set");
    let doc_json = serde_json::to_value(doc_tree.as_ref()).unwrap();
    assert_eq!(doc_json, json!({ "l": { "ft": "pdf" } }));
}

#[tokio::test]
async fn ast_endpoint_passes_through_plain_document_literal() {
    let soup = MockSoup::new();
    let inner_counter = soup.called.clone();
    let router: Router = mock_router_with(soup);

    let doc_id = Uuid::new_v4();
    let request = authenticated_request()
        .uri("/soup/ast")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&json!({
                "df": { "l": { "id": doc_id.to_string() } }
            }))
            .unwrap(),
        ))
        .unwrap();

    let _res = router.oneshot(request).await.unwrap();

    let arg = {
        let mut guard = inner_counter.lock().unwrap();
        guard
            .pop()
            .expect("SoupService::handle should have been called")
    };

    let filter: EntityFilterAst = serde_json::from_value(arg.expanded_filter).unwrap();
    let doc_tree = filter
        .document_filter
        .expect("document_filter should be set");
    let doc_json = serde_json::to_value(doc_tree.as_ref()).unwrap();
    assert_eq!(doc_json, json!({ "l": { "id": doc_id.to_string() } }));
}

#[tokio::test]
async fn ast_endpoint_passes_through_foreign_entity_filter() {
    let soup = MockSoup::new();
    let inner_counter = soup.called.clone();
    let router: Router = mock_router_with(soup);

    let request = authenticated_request()
        .uri("/soup/ast")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&json!({
                "fef": { "l": { "feid": "github:123" } }
            }))
            .unwrap(),
        ))
        .unwrap();

    let _res = router.oneshot(request).await.unwrap();

    let arg = {
        let mut guard = inner_counter.lock().unwrap();
        guard
            .pop()
            .expect("SoupService::handle should have been called")
    };

    let filter: EntityFilterAst = serde_json::from_value(arg.expanded_filter).unwrap();
    let foreign_entity_tree = filter
        .foreign_entity_filter
        .expect("foreign_entity_filter should be set");
    let foreign_entity_json = serde_json::to_value(foreign_entity_tree.as_ref()).unwrap();
    assert_eq!(
        foreign_entity_json,
        json!({ "l": { "feid": "github:123" } })
    );
}

#[test]
fn ast_cursor_without_foreign_entity_filter_deserializes() {
    let cursor_json = json!({
        "id": Uuid::new_v4(),
        "limit": 20,
        "val": {
            "sort_type": "updated_at",
            "last_val": Utc::now(),
        },
        "filter": {
            "df": { "l": { "id": Uuid::new_v4().to_string() } }
        }
    });

    let cursor: CursorWithValAndFilter<Uuid, SimpleSortMethod, ApiEntityFilterAst> =
        serde_json::from_value(cursor_json).unwrap();

    assert!(cursor.filter.foreign_entity_filter.is_none());
}

#[tokio::test]
async fn ast_endpoint_expands_file_assoc_image_to_or_tree() {
    let soup = MockSoup::new();
    let inner_counter = soup.called.clone();
    let router: Router = mock_router_with(soup);

    let request = authenticated_request()
        .uri("/soup/ast")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&json!({
                "df": { "l": { "fa": "assoc:image" } }
            }))
            .unwrap(),
        ))
        .unwrap();

    let _res = router.oneshot(request).await.unwrap();

    let arg = {
        let mut guard = inner_counter.lock().unwrap();
        guard
            .pop()
            .expect("SoupService::handle should have been called")
    };

    let filter: EntityFilterAst = serde_json::from_value(arg.expanded_filter).unwrap();
    let doc_tree = filter
        .document_filter
        .expect("document_filter should be set");

    // collect all file type strings from the expanded OR-tree
    fn collect_file_types(
        expr: &filter_ast::Expr<item_filters::ast::document::DocumentLiteral>,
        out: &mut Vec<String>,
    ) {
        match expr {
            filter_ast::Expr::Or(a, b) => {
                collect_file_types(a, out);
                collect_file_types(b, out);
            }
            filter_ast::Expr::Literal(item_filters::ast::document::DocumentLiteral::FileType(
                ft,
            )) => {
                out.push(
                    serde_json::to_value(ft)
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string(),
                );
            }
            other => panic!("unexpected node in expanded image tree: {other:?}"),
        }
    }

    let mut actual: Vec<String> = Vec::new();
    collect_file_types(doc_tree.as_ref(), &mut actual);
    actual.sort();

    let mut expected: Vec<String> = item_filters::ast::document::resolve_file_types("assoc:image")
        .into_iter()
        .map(|ft| {
            serde_json::to_value(ft)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    expected.sort();

    assert_eq!(actual, expected);
    assert!(
        actual.len() > 1,
        "image association should expand to multiple file types"
    );
}

#[tokio::test]
async fn it_can_expand_assoc_ast() {
    let js = json!({
        "df": {
            "&": [
                {
                    "l": {
                        "fa": "assoc:code"
                    }
                },
                {
                    "!": {
                        "l": {
                            "dst": "task"
                        }
                    }
                }
            ]
        },
        "ef": {
            "l": {
                "ThreadId": "00000000-0000-0000-0000-000000000000"
            }
        },
        "chanf": {
            "l": {
                "ChannelId": "00000000-0000-0000-0000-000000000000"
            }
        },
        "cf": {
            "l": {
                "cid": "00000000-0000-0000-0000-000000000000"
            }
        },
        "pf": {
            "l": {
                "pid": "00000000-0000-0000-0000-000000000000"
            }
        },
        "callf": {
            "l": {
                "ChannelId": "00000000-0000-0000-0000-000000000000"
            }
        },
        "limit": 100,
        "sort_method": "updated_at"
    });

    let soup = MockSoup::new();
    let inner_counter = soup.called.clone();
    let router: Router = mock_router_with(soup.clone());

    let request = authenticated_request()
        .uri("/soup/ast")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(serde_json::to_vec(&js).unwrap()))
        .unwrap();

    let _res = router.oneshot(request).await.unwrap();

    {
        let mut guard = inner_counter.lock().unwrap();
        guard
            .pop()
            .expect("SoupService::handle should have been called");
    }

    let filter: ApiEntityFilterAst = serde_json::from_value(js.clone()).unwrap();

    #[derive(Serialize)]
    struct Data(chrono::DateTime<Utc>, Uuid);

    impl Identify for Data {
        type Id = Uuid;

        fn id(&self) -> Uuid {
            self.1
        }
    }

    impl SortOn<SimpleSortMethod> for Data {
        fn sort_on(
            sort_type: SimpleSortMethod,
        ) -> impl FnMut(&Self) -> CursorVal<SimpleSortMethod> {
            move |v| CursorVal {
                sort_type,
                last_val: v.0,
            }
        }
    }

    let now = Utc::now();
    let res = (0..1000)
        .map(|x| Data(now - Duration::seconds(x), Uuid::new_v4()))
        .paginate_on(100, SimpleSortMethod::UpdatedAt)
        .filter_on(filter)
        .into_page();

    let cursor = res.type_erase().next_cursor.unwrap();

    let request2 = authenticated_request()
        .uri(format!("/soup/ast?cursor={cursor}"))
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&serde_json::json!({})).unwrap(),
        ))
        .unwrap();

    let router: Router = mock_router_with(soup);

    let _res = router.oneshot(request2).await.unwrap();

    let guard = inner_counter.lock().unwrap();
    let req = guard
        .first()
        .expect("SoupService::handle should have been called with next cursor");
    assert!(matches!(req.cursor_kind, MockCursorKind::SimpleCursor));
}

/// Ascending frecency has no meaning — the frecency branch orders by relevance
/// score and never applies the merged sort a direction would flip. Rejecting
/// keeps the parameter honest rather than accepting it and doing nothing.
#[tokio::test]
async fn ascending_frecency_is_rejected() {
    let request = authenticated_request()
        .uri("/soup/ast")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&json!({
                "sort_method": "frecency",
                "sort_direction": "asc"
            }))
            .unwrap(),
        ))
        .unwrap();

    let (status, body) = send_json(mock_router(), request).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body,
        json!({
            "message": "sort_direction=asc is not supported with sort_method=frecency"
        })
    );
}

/// The descending default must still reach the service — the guard is scoped to
/// the one unsupported combination, not to frecency as a whole.
#[tokio::test]
async fn descending_frecency_is_accepted() {
    let soup = MockSoup::new();
    let inner_counter = soup.called.clone();
    let router: Router = mock_router_with(soup);

    let request = authenticated_request()
        .uri("/soup/ast")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&json!({ "sort_method": "frecency" })).unwrap(),
        ))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // The mock service always errors, so the 500 is the harness rather than the
    // guard — pinned instead of loosely asserting "not 400" so a genuine
    // regression to some other status is caught. Reaching the service at all is
    // the actual claim: the guard let this combination through.
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(inner_counter.lock().unwrap().len(), 1);
}

/// touched_by_me selects the touched query mode from the plain sort param.
#[tokio::test]
async fn touched_by_me_sort_selects_touched_query() {
    let soup = MockSoup::new();
    let inner_counter = soup.called.clone();
    let router: Router = mock_router_with(soup);

    let request = authenticated_request()
        .uri("/soup/ast")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&json!({ "sort_method": "touched_by_me" })).unwrap(),
        ))
        .unwrap();

    let _res = router.oneshot(request).await.unwrap();
    let guard = inner_counter.lock().unwrap();
    let req = guard.first().unwrap();
    assert!(matches!(req.cursor_kind, MockCursorKind::TouchedSort));
}

/// A touched page cursor round-trips through the cursor extractor into the
/// touched query mode, not one of the other two cursor shapes.
#[tokio::test]
async fn touched_cursor_round_trips() {
    let soup = MockSoup::new();
    let inner_counter = soup.called.clone();
    let router: Router = mock_router_with(soup);

    // The shape the service emits: entity id + own-mutation timestamp. The
    // sort marker serializes as null (like frecency's); the timestamp value
    // is what keeps this out of the frecency cursor arm.
    let cursor = models_pagination::Base64Str::encode_json(models_pagination::Cursor {
        id: Uuid::new_v4().to_string(),
        limit: 20usize,
        val: CursorVal {
            sort_type: models_pagination::TouchedByMe,
            last_val: chrono::DateTime::<chrono::Utc>::default(),
        },
        filter: EntityFilters::default(),
    })
    .type_erase();

    // Percent-encode the base64 characters that are unsafe in a query string.
    let cursor = cursor
        .replace('+', "%2B")
        .replace('/', "%2F")
        .replace('=', "%3D");
    let request = authenticated_request()
        .uri(format!("/soup?cursor={cursor}"))
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&serde_json::json!({})).unwrap(),
        ))
        .unwrap();

    let _res = router.oneshot(request).await.unwrap();
    let guard = inner_counter.lock().unwrap();
    let req = guard.first().unwrap();
    assert!(matches!(req.cursor_kind, MockCursorKind::TouchedCursor));
}

/// Ascending touched order is rejected for the same reason as frecency: the
/// touched branch orders by the caller's own latest mutation and never
/// applies the merged sort a direction would flip.
#[tokio::test]
async fn ascending_touched_is_rejected() {
    let request = authenticated_request()
        .uri("/soup/ast")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&json!({
                "sort_method": "touched_by_me",
                "sort_direction": "asc"
            }))
            .unwrap(),
        ))
        .unwrap();

    let (status, body) = send_json(mock_router(), request).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body,
        json!({
            "message": "sort_direction=asc is not supported with sort_method=touched_by_me"
        })
    );
}

/// notified_at selects the notified query mode from the plain sort param.
#[tokio::test]
async fn notified_at_sort_selects_notified_query() {
    let soup = MockSoup::new();
    let inner_counter = soup.called.clone();
    let router: Router = mock_router_with(soup);

    let request = authenticated_request()
        .uri("/soup/ast")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&json!({ "sort_method": "notified_at" })).unwrap(),
        ))
        .unwrap();

    let _res = router.oneshot(request).await.unwrap();
    let guard = inner_counter.lock().unwrap();
    let req = guard.first().unwrap();
    assert!(matches!(req.cursor_kind, MockCursorKind::NotifiedSort));
}

/// A notified page cursor round-trips into the notified query mode. Its
/// value is a timestamp exactly like the touched cursor's, so this also
/// proves the `"notified_at"` sort marker keeps it out of the touched arm.
#[tokio::test]
async fn notified_cursor_round_trips() {
    let soup = MockSoup::new();
    let inner_counter = soup.called.clone();
    let router: Router = mock_router_with(soup);

    let cursor = models_pagination::Base64Str::encode_json(models_pagination::Cursor {
        id: Uuid::new_v4().to_string(),
        limit: 20usize,
        val: CursorVal {
            sort_type: models_pagination::NotifiedAt,
            last_val: chrono::DateTime::<chrono::Utc>::default(),
        },
        filter: EntityFilters::default(),
    })
    .type_erase();

    let cursor = cursor
        .replace('+', "%2B")
        .replace('/', "%2F")
        .replace('=', "%3D");
    let request = authenticated_request()
        .uri(format!("/soup?cursor={cursor}"))
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&serde_json::json!({})).unwrap(),
        ))
        .unwrap();

    let _res = router.oneshot(request).await.unwrap();
    let guard = inner_counter.lock().unwrap();
    let req = guard.first().unwrap();
    assert!(matches!(req.cursor_kind, MockCursorKind::NotifiedCursor));
}

/// Ascending notified order is rejected like frecency and touched: the
/// branch orders by the caller's latest notification and never applies the
/// merged sort a direction would flip.
#[tokio::test]
async fn ascending_notified_is_rejected() {
    let request = authenticated_request()
        .uri("/soup/ast")
        .method(Method::POST)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&json!({
                "sort_method": "notified_at",
                "sort_direction": "asc"
            }))
            .unwrap(),
        ))
        .unwrap();

    let (status, body) = send_json(mock_router(), request).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body,
        json!({
            "message": "sort_direction=asc is not supported with sort_method=notified_at"
        })
    );
}
