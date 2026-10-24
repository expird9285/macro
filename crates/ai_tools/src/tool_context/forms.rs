//! Forms authoring composition shared by interactive and headless tool hosts.
use super::*;
use ::forms::{
    domain::{authoring::workflow::AuthoringWorkflow, ports::FormEventPublisher},
    outbound::{
        authoring_access::EntityAuthoringAccess,
        gateway_event_publisher::{FormPublishError, GatewayFormEventPublisher},
    },
};
use collab_surface::outbound::{PgCollabSurfaceService, pg_collab_surface_service};

/// Liveness is optional only for hosts without a gateway (including isolated tests).
pub struct ToolFormEvents(Option<GatewayFormEventPublisher>);
impl FormEventPublisher for ToolFormEvents {
    type Error = FormPublishError;
    async fn form_changed(&self, id: ::forms::domain::models::FormId) -> Result<(), Self::Error> {
        if let Some(events) = &self.0 {
            events.form_changed(id).await?;
        }
        Ok(())
    }
}
type Core = ::forms::wiring::PgFormsService<
    ToolDatabasesService,
    ToolEntityAccessService,
    ToolFormEvents,
    MaybeToolEventBroker,
    PgCollabSurfaceService,
>;
/// Production Forms workflow with owning-domain validation.
pub type ToolFormsService = AuthoringWorkflow<
    Core,
    ToolDatabasesService,
    (),
    EntityAuthoringAccess<ToolEntityAccessService>,
    ::forms::outbound::authoring_editor::WorkerFormEditor,
>;
/// Forms tool context for every AI host.
pub type ToolFormsToolContext = ::forms::inbound::toolset::FormsToolContext<ToolFormsService>;

/// Host-configured origins for Forms links and collaborative editing.
pub struct FormsToolConfig {
    /// Public app origin used for returned links.
    pub app_origin: String,
    /// Existing Cloudflare AI editing worker origin.
    pub editing_worker_url: String,
}

/// Reuse host clients and the same Forms domain services as HTTP.
pub fn build_forms_tool_context(
    pool: sqlx::PgPool,
    documents: &ToolDocumentToolContext,
    databases: &ToolDatabasesToolContext,
    gateway: Option<ConnectionGatewayClient>,
    broker: MaybeToolEventBroker,
    config: FormsToolConfig,
) -> ToolFormsToolContext {
    let FormsToolConfig {
        app_origin,
        editing_worker_url,
    } = config;
    let surfaces = Arc::new(pg_collab_surface_service(
        pool.clone(),
        documents.lexical_client.as_ref().clone(),
        documents.sync_service_client.as_ref().clone(),
        documents.document_permission_jwt_secret.clone(),
    ));
    let core = Arc::new(::forms::wiring::build_service(
        pool.clone(),
        databases.service.clone(),
        databases.entity_access_service.clone(),
        ToolFormEvents(gateway.map(GatewayFormEventPublisher::new)),
        broker,
        surfaces,
    ));
    ToolFormsToolContext {
        service: Arc::new(AuthoringWorkflow {
            core,
            databases: databases.service.clone(),
            booking: (),
            access: EntityAuthoringAccess(databases.entity_access_service.clone()),
            editor: ::forms::outbound::authoring_editor::WorkerFormEditor::new(
                reqwest::Client::new(),
                editing_worker_url,
            ),
            app_origin,
        }),
        actor: databases.actor,
    }
}
