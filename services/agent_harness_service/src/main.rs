#![recursion_limit = "256"]
//! Composition root for the agent harness service.
//!
//! The hexagon lives in `crates/agent_harness`; this binary is the shell
//! around it: it builds the Postgres repositories, the in-process runtime
//! manager, and a channel service with the full side-effect stack for
//! announcements, derives agent triggers from `macro.channels`, then drives
//! the orchestrator from the resulting `macro.agent_sessions` events.

mod agent_runtime_directory;
mod api;
mod bots_directory;
mod code_mode;
mod coding_agents;
mod config;
mod containers;
mod conversation_admission;
mod external_session_requests;
mod internal_mcp;
mod model_providers;
mod permission_policy;
mod routine_sessions;
mod runtime_commands;
mod trigger;

#[cfg(test)]
mod test;

use std::{future::Future, pin::Pin, sync::Arc};

use agent_egress::domain::approval::ToolApprovalService;
use agent_egress::domain::service::EgressServiceImpl;
use agent_egress::outbound::custom_mcp::WithCustomMcp;
use agent_egress::outbound::forwarder::ReqwestForwarder;
use agent_egress::outbound::github_tokens::GithubAppTokens;
use agent_egress::outbound::macro_mcp::{MacroApiTokenSigner, WithMacroMcp};
use agent_egress::outbound::mcp_credentials::PipedreamMcpCredentials;
use agent_egress::outbound::session_authority::StoredTokenSessionAuthority;
use agent_egress::outbound::tool_approvals::{PgToolApprovalSignals, PgToolApprovalStore};
use agent_fold::domain::service::FoldedMessageService;
use agent_harness::domain::model::{
    AgentKind, AgentRuntimeConfig, HarnessCommand, HarnessDefaults, SessionDefaults,
    StaticFileLinks,
};
use agent_harness::domain::model_load::AgentModelsServiceImpl;
use agent_harness::domain::ports::AgentRuntimeDirectory as _;
use agent_harness::domain::ports::LateBoundHeldToolCallObserver;
use agent_harness::domain::service::AgentHarnessService;
use agent_harness::domain::trigger_router::{
    RoutedTrigger, agent_trigger_bot_id, route_agent_trigger,
};
use agent_harness::inbound::model_load::AgentModelsRouterState;
use agent_harness::inbound::tool_approvals::{ToolApprovalsRouterState, tool_approvals_router};
use agent_harness::outbound::agent_prompt_composer::LexicalAgentPromptComposer;
use agent_harness::outbound::channel_announcer::MessageAnnouncer;
use agent_harness::outbound::channel_prompt_context::MessagePromptContextAdapter;
use agent_harness::outbound::egress::EgressProvisioner;
use agent_harness::outbound::forward::RedisCommandForwarder;
use agent_harness::outbound::notifications::IngressAgentSessionNotifier;
use agent_harness::outbound::prompt_mentions::{LexicalPromptMentions, PgSessionAccess};
use agent_harness::outbound::tool_approvals::{
    ReleaseHeldCallsOnTurnEnd, SessionToolApprovalAnnouncer,
};
use agent_inmem::domain::engine::TurnEngine;
use agent_inmem::outbound::acp_mcp::AcpMcpConnector;
use agent_inmem::outbound::egress_mcp::EgressMcpClient;
use agent_inmem::outbound::local_attachments::StaticFileAttachmentTurnEngine;
use agent_inmem::outbound::log_frames::LogFrameSource;
use agent_inmem::outbound::manager::InMemAgentManager;
use agent_inmem::outbound::tool_catalog::McpToolCatalog;
use agent_inmem::rig_engine::RigTurnEngine;
use agent_runtime_directory::PgAgentRuntimeDirectory;
use agent_session::domain::model::{AgentMcpServers, ReplicaId};
use agent_session::domain::ports::{NoOpRealtime, SessionOwnership as _};
use agent_session::domain::service::{AgentSessionService, AgentSessionServiceImpl};
use agent_session::inbound::axum_router::{
    AgentSessionControlState, AgentSessionRouterState, CreateSessionState,
};
use agent_session::outbound::broker_lifecycle_publisher::BrokerLifecyclePublisher;
use agent_session::outbound::connection_gateway_realtime::ConnectionGatewayAgentSessionRealtime;
use agent_session::outbound::name_generator::HaikuAgentSessionNameGenerator;
use agent_session::outbound::postgres::PgAgentSessionRepo;
use agent_session_realtime::outbound::redis_stream::publisher::RedisLogStreamRealtime;
use agent_trigger::domain::broker_events::AgentSessionMacroEvent;
use anyhow::Context as _;
use bots::outbound::pg_bots_repo::PgBotsRepo;
use bots_directory::PgBotDirectory;
use channels::domain::side_effects::{ChannelSideEffectService, SpawnedChannelEventDispatcher};
use channels::outbound::connection_gateway_realtime::ConnectionGatewayChannelRealtimePublisher;
use channels::outbound::contacts_dispatcher::ContactsChannelDispatcher;
use channels::outbound::notification_sender::NotificationChannelSender;
use channels::outbound::pg_channels_repo::PgChannelsRepo;
use channels::outbound::pg_side_effect_context::PgChannelSideEffectContext;
use config::{Config, Environment};
use connection_gateway_client::ConnectionGatewayClient;
use containers::{InMemRuntime, RoutedContainers};
use entity_registry::OwnerGrantPolicy;
use entity_registry_db_utils::OwnedEntityRegistrar;
use github::domain::service::{InstallationTokenConfig, InstallationTokenService};
use github::outbound::github_sync_client::GithubSyncClientImpl;
use github::outbound::pg_github_sync_repo::PgGithubSyncRepo;
use kafka_util::{GroupName, KafkaEventConsumer, consumer_span, record_span_error};
use lexical_client::LexicalClient;
use macro_auth::middleware::decode_jwt::JwtValidationArgs;
use macro_authorization::{
    InternalAuthConfig, MacroAuthJwtValidator, MacroAuthorizationServiceImpl,
    MacroAuthorizationState, PgBotAuthorizationRepo, PgBotAuthorizer,
    PgUserApiKeyAuthorizationRepo, PgUserApiKeyAuthorizer,
};
use macro_entrypoint::{MacroEntrypoint, shutdown_signal};
use macro_event_broker::{
    KafkaConsumerAdapter, KafkaEventPublisher, MacroEvent as _, MacroEventBrokerService,
    MacroEventCollection as _, MacroEventConsumerService,
};
use macro_service_urls::{
    AgentHarnessEgressUrl, ConnectionGatewayUrl, LexicalServiceUrl, McpServiceUrl,
    StaticFilePublicUrl, StaticFileServiceUrl,
};
use mcp_client::domain::models::AesKey;
use mcp_client::outbound::pg_server_repo::PgServerRepo;
use model_providers::InMemoryModels;
use permission_policy::PgPermissionPolicySource;
use pipedream_mcp::outbound::api::{PipedreamClient, PipedreamConfig};
use pipedream_mcp::outbound::pg_connection_repo::PgConnectionRepo;
use rdkafka::consumer::CommitMode;
use rdkafka::message::{BorrowedMessage, Message as _};
use sqlx::postgres::PgPoolOptions;
use tokio_retry::{Retry, strategy::FixedInterval};
use tracing::Instrument as _;

use agent_session::domain::ports::{NoOpAgentSessionNameGenerator, NoOpTurnObserver};
use runtime_commands::consume_runtime_commands;

/// Consumer group owning this harness's agent-session offsets.
struct AgentHarnessConsumerGroup;

const RUNTIME_COMMAND_CONSUMER_ATTEMPTS: usize = 5;

impl GroupName for AgentHarnessConsumerGroup {
    const GROUP_NAME: &'static str = "agent-harness-service";
}

macro_event_broker::declare_topics!(DeclaredMacroEvent: AgentSessionMacroEvent);

type HarnessKafkaAdapter = KafkaConsumerAdapter<AgentHarnessConsumerGroup, DeclaredMacroEvent>;
type HarnessConsumer = MacroEventConsumerService<DeclaredMacroEvent, HarnessKafkaAdapter>;

fn commit_message(consumer: &HarnessConsumer, message: &BorrowedMessage<'_>) -> anyhow::Result<()> {
    consumer
        .inner()
        .commit_message(message, CommitMode::Sync)
        .map_err(|error| anyhow::anyhow!("failed to commit agent session offset: {error:?}"))
}

type HarnessWork =
    Pin<Box<dyn Future<Output = agent_harness::domain::error::Result<()>> + Send + 'static>>;

struct PendingHarnessWork {
    session_id: agent_session::domain::model::AgentSessionId,
    span: tracing::Span,
    work: HarnessWork,
    description: &'static str,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let entrypoint = MacroEntrypoint::default().init();
    let result = run().await;
    entrypoint.shutdown();
    result
}

fn macro_mcp_endpoint(base_url: &McpServiceUrl) -> Result<url::Url, url::ParseError> {
    // Append rather than Url::join("/mcp"), which would discard the gateway prefix.
    url::Url::parse(&format!("{}/mcp", base_url.trim_end_matches('/')))
}

// Share the exact authenticated internal router between the listener and the
// in-process MCP transport; internal tools are not third-party egress apps.
fn internal_mcp_handler(
    app: axum::Router,
) -> agent_inmem::outbound::egress_mcp::InternalMcpHandler {
    use http_body_util::BodyExt as _;
    use tower::ServiceExt as _;
    Arc::new(move |request| {
        let app = app.clone();
        Box::pin(async move {
            let response = match app.oneshot(request.map(axum::body::Body::new)).await {
                Ok(response) => response,
                Err(never) => match never {},
            };
            response.map(|body| {
                body.map_err(|error| -> agent_egress::domain::model::BoxError { Box::new(error) })
                    .boxed_unsync()
            })
        })
    })
}

async fn run() -> anyhow::Result<()> {
    agent_harness::install_tls_provider();
    // AWS first, because the config's secrets resolve through Secrets Manager.
    let aws_config = macro_aws_config::get_macro_aws_config().await;
    let secrets = secretsmanager_client::SecretsManager::new(aws_sdk_secretsmanager::Client::new(
        &aws_config,
    ));
    let config = Config::from_env()?
        .resolve_remote_secrets(Environment::new_or_prod(), &secrets)
        .await
        .context("failed to resolve agent harness service secrets")?;
    let non_user_owners = config.non_user_owners()?;
    let enable_dev_commands = matches!(
        config.environment,
        Environment::Local | Environment::Develop
    );
    let inmem_bot = bot_id::MACRO_NEW_BOT_ID;

    let pool = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(5)
        .connect(config.database_url.as_ref())
        .await
        .context("failed to connect to macrodb")?;

    let connection_gateway = Arc::new(ConnectionGatewayClient::new(
        config.internal_api_key.clone(),
        ConnectionGatewayUrl::new()?.to_string(),
    ));

    let session_registrar =
        OwnedEntityRegistrar::new(OwnerGrantPolicy::new(PgBotsRepo::new(pool.clone())));
    let session_list_policy = agent_session::domain::list_visibility::SessionListPolicy {
        hide_agent_dms: config.hide_agent_dm_sessions,
    };
    let session_repo = PgAgentSessionRepo::new(pool.clone(), session_registrar.clone())
        .with_list_policy(session_list_policy);
    session_list_policy.reconcile(&session_repo).await?;
    let entity_access = Arc::new(
        entity_access::domain::service::EntityAccessServiceImpl::new(
            entity_access::outbound::PgAccessRepository::new(pool.clone()),
        ),
    );
    let session_audience = agent_session::domain::audience::AuthorizedSessionAudience::new(
        session_repo.clone(),
        (*entity_access).clone(),
        agent_session::outbound::connection_gateway_realtime::ConnectionGatewaySessionSubscriptions(
            connection_gateway.clone(),
        ),
    );
    let replica = ReplicaId::mint();
    tracing::info!(%replica, "harness replica identity");
    let turn_observer = Arc::new(agent_session::domain::ports::LateBoundTurnObserver::new());
    let broker = MacroEventBrokerService::new(
        KafkaEventPublisher::new(config.kafka_brokers.as_ref())
            .context("failed to create kafka event publisher")?,
        macro_event_broker::GlobalSpawner,
    );
    let notifications = Arc::new(notification::domain::service::SqsNotificationIngress {
        queue: notification::outbound::queue::SqsQueue::new(
            aws_sdk_sqs::Client::new(&aws_config),
            macro_queues::NotificationIngressQueue::new().to_string(),
        ),
    });
    let admission = ai_billing::composition::pg_admission_service(
        pool.clone(),
        config.enable_ai_usage_enforcement,
        config.ai_pricing(),
    );
    let recorder = ai_billing::composition::pg_settling_recorder(
        pool.clone(),
        config.enable_ai_usage_enforcement,
        config.ai_pricing(),
        config.settlement_route()?,
    );
    let lifecycle_publisher = Arc::new(BrokerLifecyclePublisher::new(broker.clone()));
    let redis = redis::Client::open(config.redis_uri.as_ref())
        .context("failed to create the shared Redis client")?;
    let sessions = AgentSessionServiceImpl::new(
        session_repo.clone(),
        FoldedMessageService::new(session_repo.clone()),
        RedisLogStreamRealtime::new(
            ConnectionGatewayAgentSessionRealtime::new(
                connection_gateway.clone(),
                session_audience.clone(),
            ),
            redis.clone(),
        ),
        agent_session::domain::name_generation::AdmittedAgentSessionNameGenerator::new(
            HaikuAgentSessionNameGenerator::new(recorder.clone()),
            admission.clone(),
        ),
        turn_observer.clone(),
        lifecycle_publisher.clone(),
        replica,
    );

    let pending_commands = agent_harness::domain::pending::PendingCommands::new();

    // Channel attachments reach a prompt as links the agent can fetch, so
    // the trigger router needs to know where static files are served from.
    let static_file_links = StaticFileLinks::new(StaticFileServiceUrl::new()?.to_string());

    // Tracks event publishes the in-memory agent's tool context starts;
    // closed and drained on shutdown so nothing is dropped mid-publish.
    let event_broker_tracker = tokio_util::task::TaskTracker::new();
    let mcp_connections = Arc::new(PgConnectionRepo::new(pool.clone()));
    let mcp_credentials_key = AesKey::try_from(config.mcp_credentials_key_secret_name.as_ref())
        .context("invalid MCP credentials encryption key")?;
    let mcp_servers = Arc::new(PgServerRepo::new(pool.clone(), mcp_credentials_key));

    let pipedream = PipedreamClient::new(PipedreamConfig {
        client_id: config.pipedream_client_id.to_string(),
        client_secret: config.pipedream_client_secret.to_string(),
        project_id: config.pipedream_project_id.to_string(),
        environment: config.pipedream_environment.clone(),
        api_url: config.pipedream_api_url.clone(),
        mcp_url: config.pipedream_mcp_url.clone(),
        allowed_origins: Vec::new(),
    })
    .context("failed to build Pipedream client")?;

    let mcp_credentials = WithMacroMcp::new(
        WithCustomMcp::new(
            PipedreamMcpCredentials::new(Arc::clone(&mcp_connections), pipedream),
            Arc::clone(&mcp_servers),
        ),
        MacroApiTokenSigner::new(
            pool.clone(),
            config.macro_api_token_issuer.as_ref(),
            config.macro_api_token_private_secret_key.as_ref(),
        ),
        macro_mcp_endpoint(&McpServiceUrl::new()?).context("MCP service endpoint is not a URL")?,
        matches!(config.environment, Environment::Local),
    )
    .context("the macro MCP upstream is misconfigured")?;

    let held_tool_calls = Arc::new(LateBoundHeldToolCallObserver::new());
    let tool_approvals = Arc::new(ToolApprovalService::new(
        PgToolApprovalStore::new(pool.clone()),
        PgToolApprovalSignals::spawn(pool.clone()),
        SessionToolApprovalAnnouncer::new(
            sessions.clone(),
            IngressAgentSessionNotifier::new(Arc::clone(&notifications)),
            held_tool_calls.clone(),
        ),
    ));
    let egress = Arc::new(
        EgressServiceImpl::new(
            StoredTokenSessionAuthority::new(PgAgentSessionRepo::new(
                pool.clone(),
                session_registrar,
            )),
            mcp_credentials,
            GithubAppTokens::new(InstallationTokenService::new(
                InstallationTokenConfig {
                    client_id: config.github_sync_app_client_id.clone(),
                    private_key_pem: config.github_sync_app_pem_secret_key.as_ref().to_owned(),
                },
                PgGithubSyncRepo::new(pool.clone()),
                GithubSyncClientImpl::default(),
            )),
            ReqwestForwarder::new()?,
        )
        .with_owner_approvals(Arc::clone(&tool_approvals)),
    );

    let egress_base_url = AgentHarnessEgressUrl::new()?.to_string();

    let tool_context = ai_tools::build_tool_service_context_from_env(
        pool.clone(),
        event_broker_tracker.clone(),
        config.enable_ai_usage_enforcement,
        config.ai_pricing(),
        recorder.clone(),
    )
    .await
    .context("failed to build the in-memory agent tool context")?;
    let task_directory = agent_harness::outbound::session_tasks::DocumentTaskDirectory::new(
        tool_context.document_tool_context.service.clone(),
        tool_context
            .document_tool_context
            .entity_access_service
            .clone(),
    );
    let owner_tool_gate: Arc<dyn agent_inmem::domain::tool_gate::NativeToolGate> = Arc::new(
        agent_inmem::outbound::approval_gate::OwnerApprovalGate::new(
            session_repo.clone(),
            Arc::clone(&tool_approvals),
        ),
    );
    let code_scratch = tempfile::tempdir()?;
    let code_runner = match &config.deno_binary {
        Some(binary) => {
            let runner = code_execution::outbound::deno::DenoRunner::new(
                binary.into(),
                code_scratch.path().to_owned(),
                128,
            )
            .await
            .map_err(|error| anyhow::anyhow!("invalid Deno runtime: {error}"))?;
            Some(code_execution::domain::ExecutionService::new(
                Arc::new(runner),
                code_execution::domain::Limits::default(),
            )?)
        }
        None => None,
    };
    let code_executor = code_runner
        .as_ref()
        .map(|runner| Arc::new(runner.clone()) as Arc<dyn code_execution::domain::ProgramExecutor>);
    let code_mode_enabled = code_executor.is_some();
    let code_mode: Arc<dyn agent_code_mode::domain::SessionCodeMode> =
        Arc::new(agent_code_mode::domain::CodeModeService::new(
            code_executor,
            Arc::new(code_mode::RunCredentials {
                signer: MacroApiTokenSigner::new(
                    pool.clone(),
                    config.macro_api_token_issuer.as_ref(),
                    config.macro_api_token_private_secret_key.as_ref(),
                ),
                gate: owner_tool_gate.clone(),
                hosts: code_mode::sdk_hosts()?,
            }),
            Arc::new(agent_code_mode::outbound::postgres::PgExecutionStore::new(
                pool.clone(),
            )),
            Arc::new(agent_code_mode::outbound::turns::SessionTurns(
                session_repo.clone(),
            )),
        ));
    let session_metadata_realtime = ConnectionGatewayAgentSessionRealtime::new(
        connection_gateway.clone(),
        session_audience.clone(),
    );
    let session_tasks: Arc<dyn agent_session::domain::session_task::SessionTasks> = Arc::new(
        agent_session::domain::session_task::SessionTaskService::new(
            session_repo.clone(),
            session_repo.clone(),
            task_directory,
            agent_harness::outbound::session_tasks::GithubTaskPullRequestLinker::new(
                github::domain::service::PullRequestTaskLinkService::new(PgGithubSyncRepo::new(
                    pool.clone(),
                )),
            ),
            session_metadata_realtime.clone(),
            macro_service_urls::AppServiceUrl::new()?.as_ref(),
        ),
    );
    let session_pull_requests: Arc<dyn agent_session::domain::pull_request::SessionPullRequests> =
        Arc::new(
            agent_session::domain::pull_request::SessionPullRequestService::new(
                session_repo.clone(),
                session_metadata_realtime,
                session_tasks.clone(),
            ),
        );
    let reviews_cell = Arc::new(std::sync::OnceLock::new());
    let internal_mcp = internal_mcp::router(
        Arc::new(session_repo.clone()),
        session_pull_requests,
        session_tasks,
        url::Url::parse(&egress_base_url)?
            .host_str()
            .context("egress URL needs a host")?
            .to_owned(),
        code_mode_enabled.then(|| code_mode.clone()),
        reviews_cell.clone(),
    );
    let mcp_connector = Arc::new(AcpMcpConnector::new(
        EgressMcpClient::new(Arc::clone(&egress), &egress_base_url)
            .with_internal_mcp(internal_mcp_handler(internal_mcp.clone())),
    ));
    let tool_catalog: Arc<dyn agent_session::domain::ports::SessionToolCatalog> =
        Arc::new(McpToolCatalog::new(Arc::clone(&mcp_connector)));
    let sessions = sessions.with_tool_catalog(Arc::clone(&tool_catalog));

    let mut inmem_engine =
        RigTurnEngine::new(pool.clone(), tool_context.clone()).with_gate(owner_tool_gate);
    if code_mode_enabled && let Some(key) = config.posthog_api_key.value() {
        let flags = feature_flags::outbound::posthog::PostHogFeatureFlags::new(
            config
                .posthog_host
                .value()
                .unwrap_or("https://us.i.posthog.com"),
            key.to_owned(),
            std::time::Duration::from_secs(3),
        )
        .map_err(|error| anyhow::anyhow!("invalid feature flag configuration: {error}"))?;
        inmem_engine = inmem_engine.with_tool_mode(Arc::new(
            agent_inmem::domain::tool_mode::FlaggedToolMode::new(flags),
        ));
    }
    let inmem_model_engine: Arc<dyn TurnEngine> = Arc::new(inmem_engine);
    let public_base = StaticFilePublicUrl::new()?.parse_url()?;
    let cdn_base = match (&config.environment, macro_aws_config::LocalAwsUrl::new()) {
        (Environment::Local, Some(local_aws)) => format!(
            "{}/{}",
            local_aws.as_ref().trim_end_matches('/'),
            config::StaticStorageBucket::new()?.as_ref(),
        ),
        _ => public_base.as_str().trim_end_matches('/').to_owned(),
    };
    let attachments = static_file::inbound::attachment::StaticFileAttachmentService::new(Arc::new(
        static_file::outbound::CdnStaticFileRepo::new(cdn_base),
    ));
    let inmem_model_engine: Arc<dyn TurnEngine> = Arc::new(StaticFileAttachmentTurnEngine::new(
        inmem_model_engine,
        public_base,
        attachments,
    )?);
    let frames = Arc::new(LogFrameSource::new(session_repo.clone()));
    let permissions_repo = roles_and_permissions::outbound::pgpool::MacroDB::new(pool.clone());
    let inmem_model_access: Arc<dyn agent_inmem::domain::model_access::InMemModelAccess> = Arc::new(
        agent_inmem::domain::model_access::PermissionModelAccess::new(
            roles_and_permissions::domain::service::UserRolesAndPermissionsServiceImpl::new(
                permissions_repo.clone(),
                permissions_repo,
            ),
        ),
    );
    let inmem = InMemRuntime {
        manager: InMemAgentManager::new(
            Arc::clone(&inmem_model_engine),
            frames,
            mcp_connector,
            Arc::clone(&inmem_model_access),
        )
        .with_admission(admission.clone())
        .with_dev_commands(enable_dev_commands),
    };
    let inmem_sessions = AgentSessionServiceImpl::new(
        session_repo.clone(),
        FoldedMessageService::new(session_repo.clone()),
        NoOpRealtime,
        NoOpAgentSessionNameGenerator,
        turn_observer.clone(),
        lifecycle_publisher.clone(),
        replica,
    )
    .with_tool_catalog(Arc::clone(&tool_catalog));
    let containers = RoutedContainers::new(Some(inmem), inmem_sessions);

    let fixed_runtimes = vec![(
        inmem_bot,
        AgentRuntimeConfig {
            kind: AgentKind::InMemory,
            model: config.inmem_model.clone(),
            harness: config.inmem_harness_slug.clone(),
            instructions: String::new(),
            mcp_servers: AgentMcpServers::OwnerConnections,
        },
    )];
    let runtime_directory =
        PgAgentRuntimeDirectory::new(PgBotsRepo::new(pool.clone()), fixed_runtimes.clone());
    tracing::info!(
        bots = ?fixed_runtimes.iter().map(|(bot, _)| bot.as_uuid()).collect::<Vec<_>>(),
        in_process_bot = %inmem_bot.as_uuid(),
        environment = %config.environment,
        "agent harness serving bots"
    );

    let contacts_ingress = Arc::new(contacts::domain::service::SqsContactsIngress {
        queue: contacts::outbound::ingress::SqsContactsQueue::new(
            aws_sdk_sqs::Client::new(&aws_config),
            macro_queues::ContactsQueue::new().to_string(),
        ),
    });
    let side_effects = ChannelSideEffectService::new(
        PgChannelSideEffectContext::new(pool.clone()),
        ConnectionGatewayChannelRealtimePublisher::new(connection_gateway.clone()),
        NotificationChannelSender::new(Arc::clone(&notifications)),
        ContactsChannelDispatcher::new(contacts_ingress),
    )
    .with_macro_event_broker(broker.clone());
    let lexical = LexicalClient::new(
        config.internal_api_key.clone(),
        LexicalServiceUrl::new()?.to_string(),
    );
    let message_realtime = messages::outbound::connection_gateway::ConnectionGatewayMessages(
        connection_gateway.clone(),
    );
    let message_delivery = messages::domain::delivery::ParentMessagePublisher::new(
        channels::domain::message_delivery::ChannelMessageDelivery::new(
            PgChannelsRepo::new(pool.clone()),
            SpawnedChannelEventDispatcher::new(side_effects),
            channels::domain::service::NoopChannelReferenceSharePermissions,
            message_realtime.clone(),
        ),
        messages::domain::delivery::DiscussionDelivery::new(
            messages::outbound::pg_discussion_context::PgDiscussionContext(pool.clone())
                .with_initiatives(
                    initiative::domain::lookup::InitiativeLookup::new(
                        initiative::outbound::PgInitiativeRepo::new(pool.clone()),
                    ),
                    ai_tools::build_properties_service(pool.clone(), entity_access.clone()),
                )
                .with_crm(crm::outbound::lookup::PgCrmParentReader::new(pool.clone())),
            messages::outbound::entity_access_audience::EntityAccessMessageAudience(
                (*entity_access).clone(),
            ),
            message_realtime,
            messages::outbound::notification_sender::MessageNotificationSender(Arc::clone(
                &notifications,
            )),
        )
        .with_sharing(messages::outbound::pg_discussion_context::PgDiscussionContext(pool.clone())),
    );
    let message_service: Arc<dyn messages::domain::api::MessageServiceApi> = Arc::new(
        messages::domain::service::MessageService::new(
            messages::outbound::pg_message_repo::PgMessageRepository::new(pool.clone())
                .with_initiatives(initiative::domain::lookup::InitiativeLookup::new(
                    initiative::outbound::PgInitiativeRepo::new(pool.clone()),
                ))
                .with_crm(crm::outbound::lookup::PgCrmParentReader::new(pool.clone())),
            messages::domain::effects::MessageEffects::new(
                messages::outbound::broker::BrokerMessagePublisher::new(broker.clone()),
                messages::domain::ports::NoMessageEventPublisher,
                message_delivery,
            ),
        )
        .with_group_recipients(channels::domain::group_mentions::ChannelGroupRecipients(
            PgChannelsRepo::new(pool.clone()),
        ))
        .with_mention_extractor(lexical_mention_extractor::LexicalMentionExtractor::new(
            Arc::new(lexical.clone()),
        ))
        .with_references(
            messages::outbound::entity_access_audience::EntityAccessMessageReferences(
                (*entity_access).clone(),
            ),
        ),
    );
    let announcer = MessageAnnouncer::new(
        message_service.clone(),
        Arc::clone(&entity_access),
        lexical.clone(),
    );
    let prompt_mentions =
        LexicalPromptMentions::new(lexical.clone(), PgSessionAccess::new(pool.clone()));
    let prompt_context =
        MessagePromptContextAdapter::new(message_service.clone(), Arc::clone(&entity_access));
    let prompt_composer = LexicalAgentPromptComposer::new(lexical);

    let defaults = HarnessDefaults::new(SessionDefaults {
        bot_id: inmem_bot,
        model: config.inmem_model.clone(),
        harness: config.inmem_harness_slug.clone(),
        repo_url: None,
    });

    let draining_sessions = sessions.clone();
    let conversation_turns = Arc::new(
        agent_harness::outbound::conversation_turns::PgConversationTurnStore::new(pool.clone()),
    );
    let conversations_service = Arc::new(
        agent_harness::domain::conversations::AgentConversationsService::new(
            PgChannelsRepo::new(pool.clone()),
            bots::domain::service::BotServiceImpl::new(
                PgBotsRepo::new(pool.clone()),
                macro_event_broker::NoopMacroEventBroker,
            ),
            PgBotsRepo::new(pool.clone()),
            session_repo.clone(),
        )
        .with_turns(conversation_turns.clone()),
    );
    let harness = Arc::new(
        AgentHarnessService::new(
            Some(conversations_service.clone()),
            sessions,
            containers,
            announcer,
            prompt_context,
            prompt_composer,
            EgressProvisioner::new(
                Arc::clone(&mcp_connections),
                Arc::clone(&mcp_servers),
                egress_base_url,
            ),
            RedisCommandForwarder::new(redis.clone()),
            PgPermissionPolicySource::new(PgBotsRepo::new(pool.clone())),
            defaults,
            ReleaseHeldCallsOnTurnEnd::new(
                Arc::clone(&lifecycle_publisher),
                Arc::clone(&tool_approvals),
            ),
            pending_commands,
            prompt_mentions,
            IngressAgentSessionNotifier::new(Arc::clone(&notifications)),
        )
        .with_conversation_turns(conversation_turns)
        .with_admission(admission.clone())
        .with_warm_sessions(Arc::new(session_repo.clone()), tool_catalog),
    );
    let reviews = agent_code_mode::domain::reviews::ReviewService::new(
        Arc::new(agent_code_mode::outbound::reviews::PgReviewStore(
            pool.clone(),
        )),
        Arc::new(agent_code_mode::outbound::postgres::PgExecutionStore::new(
            pool.clone(),
        )),
        Arc::new(code_mode::reviews::Actions {
            sessions: session_repo.clone(),
            harness: harness.clone(),
            context: tool_context.clone(),
            tools: ai_tools::code_mode_tools(),
        }),
    );
    reviews_cell
        .set(reviews.clone())
        .map_err(|_| anyhow::anyhow!("Review service was initialized twice"))?;
    let warm_cleanup = Arc::clone(&harness);
    let session_visibility_repo = session_repo.clone();
    tokio::spawn(async move {
        let mut ticks = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            ticks.tick().await;
            warm_cleanup.reap_warm_sessions().await;
            if let Err(error) = session_list_policy
                .reconcile(&session_visibility_repo)
                .await
            {
                tracing::warn!(?error, "could not reconcile agent DM session visibility");
            }
        }
    });

    // Close the loop: turn ends observed by the session actors drain the
    // harness's prompt queue.
    turn_observer.bind(harness.clone());
    held_tool_calls.bind(harness.clone());
    let runtime_command_redis = redis.clone();
    let runtime_command_harness = harness.clone();
    let (runtime_commands_ready, mut runtime_commands_readiness) =
        tokio::sync::watch::channel(false);
    let runtime_commands = tokio::spawn(async move {
        let result = Retry::start(
            FixedInterval::new(std::time::Duration::from_secs(1))
                .take(RUNTIME_COMMAND_CONSUMER_ATTEMPTS - 1),
            || {
                runtime_commands_ready.send_replace(false);
                consume_runtime_commands(
                    runtime_command_redis.clone(),
                    replica,
                    runtime_command_harness.clone(),
                    runtime_commands_ready.clone(),
                )
            },
        )
        .await;
        if let Err(error) = result {
            tracing::error!(
                error = ?error,
                "runtime command Redis consumer stopped after five attempts"
            );
        }
    });
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        runtime_commands_readiness.wait_for(|ready| *ready),
    )
    .await
    .context("timed out subscribing to the runtime command bus")?
    .context("runtime command bus stopped before subscribing")?;

    let authorization_service = MacroAuthorizationServiceImpl::new(
        MacroAuthJwtValidator::new(
            JwtValidationArgs::new_with_secret_manager(config.environment, &secrets).await?,
        ),
        InternalAuthConfig {
            api_key: config.internal_api_key.clone(),
            default_user_id: None,
        },
        PgBotAuthorizer::new(PgBotAuthorizationRepo::new(pool.clone())),
        PgUserApiKeyAuthorizer::new(PgUserApiKeyAuthorizationRepo::new(pool.clone())),
    );
    let capabilities = agent_harness::inbound::capability_discovery::agent_capabilities_router(
        agent_harness::inbound::capability_discovery::AgentCapabilitiesRouterState::new(
            Arc::new(
                agent_harness::domain::capability_discovery::AgentCapabilitiesServiceImpl::new(
                    InMemoryModels::new(
                        Some(Arc::clone(&inmem_model_engine)),
                        config.inmem_model.clone(),
                        Arc::clone(&inmem_model_access),
                    ),
                ),
            ),
            MacroAuthorizationState::new(Arc::new(authorization_service.clone())),
        ),
    );
    let model_service = Arc::new(AgentModelsServiceImpl::new(InMemoryModels::new(
        Some(inmem_model_engine),
        config.inmem_model.clone(),
        inmem_model_access,
    )));
    let model_state = AgentModelsRouterState::new(
        model_service,
        MacroAuthorizationState::new(Arc::new(authorization_service.clone())),
    );
    let read_state = AgentSessionRouterState::new(
        AgentSessionServiceImpl::new(
            session_repo.clone(),
            FoldedMessageService::new(session_repo.clone()),
            ConnectionGatewayAgentSessionRealtime::new(
                connection_gateway.clone(),
                session_audience.clone(),
            ),
            NoOpAgentSessionNameGenerator,
            Arc::new(NoOpTurnObserver),
            lifecycle_publisher,
            ReplicaId::mint(),
        )
        .with_view_access(Arc::new(
            agent_session::domain::audience::EntityAccessSessionView::new((*entity_access).clone()),
        )),
        entity_access.clone(),
        MacroAuthorizationState::new(Arc::new(authorization_service.clone())),
    );
    let control_state = AgentSessionControlState::new(
        harness.clone(),
        entity_access.clone(),
        MacroAuthorizationState::new(Arc::new(authorization_service.clone())),
    );
    let bots_directory = Arc::new(PgBotDirectory::new(PgBotsRepo::new(pool.clone())));
    let create_state = CreateSessionState::new(
        harness.clone(),
        bots_directory.clone(),
        Arc::new(
            external_session_requests::BrokerExternalSessionRequests::new(
                broker.clone(),
                session_repo.clone(),
            ),
        ),
        MacroAuthorizationState::new(Arc::new(authorization_service.clone())),
        non_user_owners,
    );
    let routine_sessions = routine_sessions::router(
        (*bots_directory).clone(),
        (*harness).clone(),
        draining_sessions.clone(),
        broker.clone(),
        session_repo.clone(),
        MacroAuthorizationState::new(Arc::new(authorization_service.clone())),
    );
    let coding_agents = coding_agents::router(
        coding_agents::AvailableCodingAgents::new(bots::domain::service::BotServiceImpl::new(
            PgBotsRepo::new(pool.clone()),
            broker.clone(),
        )),
        agent_session::domain::routines::RoutineSessionsService::new(
            (*bots_directory).clone(),
            (*harness).clone(),
            external_session_requests::BrokerExternalSessionRequests::new(
                broker.clone(),
                session_repo.clone(),
            ),
            draining_sessions.clone(),
            (*harness).clone(),
        ),
        MacroAuthorizationState::new(Arc::new(authorization_service.clone())),
    );
    let http_runtime_commands_readiness = runtime_commands_readiness.clone();
    let http_port = config.port;
    let pull_requests =
        agent_session::inbound::axum_router::pull_requests::agent_session_pull_request_router(
            AgentSessionRouterState::new(
                agent_session::domain::pull_request_links::SessionPullRequestLinkService::new(
                    session_repo.clone(),
                    Arc::new(
                        agent_session::domain::audience::EntityAccessSessionView::new(
                            (*entity_access).clone(),
                        ),
                    ),
                ),
                entity_access.clone(),
                MacroAuthorizationState::new(Arc::new(authorization_service.clone())),
            ),
        );
    let code_executions = agent_code_mode::inbound::axum_router::code_execution_router(
        agent_code_mode::inbound::axum_router::CodeExecutionRouterState::new(
            code_mode,
            reviews,
            entity_access.clone(),
            MacroAuthorizationState::new(Arc::new(authorization_service.clone())),
        ),
    );
    let sharing = agent_session::inbound::axum_router::sharing::agent_session_sharing_router(
        AgentSessionRouterState::new(
            agent_session::domain::sharing::SessionSharingService::new(session_repo.clone()),
            entity_access.clone(),
            MacroAuthorizationState::new(Arc::new(authorization_service.clone())),
        ),
    );
    let conversations = agent_harness::inbound::conversations::agent_conversation_router(
        agent_harness::inbound::conversations::AgentConversationRouterState::new(
            conversations_service,
            MacroAuthorizationState::new(Arc::new(authorization_service.clone())),
        ),
    );
    let tool_approval_answers = tool_approvals_router(ToolApprovalsRouterState::new(
        tool_approvals,
        entity_access.clone(),
        MacroAuthorizationState::new(Arc::new(authorization_service.clone())),
    ));
    let http = tokio::spawn(async move {
        if let Err(error) = api::setup_and_serve(
            api::ApiStates::new(read_state, control_state, create_state, model_state)
                .with_sharing(sharing)
                .with_routine_sessions(routine_sessions)
                .with_conversations(conversations)
                .with_coding_agents(coding_agents)
                .with_capabilities(capabilities)
                .with_pull_requests(pull_requests)
                .with_tool_approvals(tool_approval_answers)
                .with_code_executions(code_executions),
            http_runtime_commands_readiness,
            http_port,
            shutdown_signal(),
        )
        .await
        {
            tracing::error!(error = ?error, "agent harness service http stopped");
        }
    });

    let heartbeat_repo = session_repo.clone();
    let heartbeat_readiness = runtime_commands_readiness.clone();
    let heartbeat = tokio::spawn(async move {
        let mut ticker =
            tokio::time::interval(agent_session::domain::ports::REPLICA_HEARTBEAT_INTERVAL);
        loop {
            ticker.tick().await;
            if !*heartbeat_readiness.borrow() {
                continue;
            }
            if let Err(error) = heartbeat_repo.heartbeat(replica, None).await {
                tracing::warn!(error = ?error, %replica, "failed to heartbeat harness replica");
            }
        }
    });

    let typing_harness = harness.clone();
    let typing_heartbeat = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(3));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            typing_harness.refresh_typing().await;
        }
    });

    let recovery_harness = harness.clone();
    let dm_recovery = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(10));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(error) = recovery_harness.recover_conversations().await {
                tracing::error!(
                    ?error,
                    "failed to reconcile durable agent conversation turns"
                );
            }
        }
    });

    let recovery_repo = session_repo.clone();
    let recovery_realtime = ConnectionGatewayAgentSessionRealtime::new(
        connection_gateway.clone(),
        session_audience.clone(),
    );
    let recovery = tokio::spawn(async move {
        let mut ticker =
            tokio::time::interval(agent_session::domain::ports::REPLICA_HEARTBEAT_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(error) = agent_session::domain::recovery::recover_stale_sessions(
                &recovery_repo,
                &recovery_realtime,
                std::num::NonZeroUsize::new(100).expect("nonzero recovery batch"),
            )
            .await
            {
                tracing::warn!(error = ?error, "failed to recover abandoned agent sessions");
            }
        }
    });

    let mut trigger = tokio::spawn(trigger::supervise(
        pool.clone(),
        config.kafka_brokers.as_ref().to_owned(),
        config.internal_api_key.clone(),
        config.document_storage_service_auth_key.clone(),
        trigger::TriggerServices {
            recorder,
            admission,
            messages: message_service,
        },
    ));

    let egress_port = config.egress_port;
    let egress_http = tokio::spawn(async move {
        if let Err(error) =
            api::serve_egress(egress, internal_mcp, egress_port, shutdown_signal()).await
        {
            tracing::error!(error = ?error, "agent harness service egress stopped");
        }
    });

    let consumer =
        KafkaEventConsumer::<AgentHarnessConsumerGroup>::from_env(config.kafka_brokers.as_ref())?;
    let consumer = KafkaConsumerAdapter::<AgentHarnessConsumerGroup, ()>::new(consumer)
        .subscribe::<DeclaredMacroEvent>()
        .map_err(|error| {
            anyhow::anyhow!("failed to subscribe to agent session events: {error:?}")
        })?;
    let consumer = HarnessConsumer::new(consumer);

    tracing::info!(
        topics = ?DeclaredMacroEvent::topics(),
        group = AgentHarnessConsumerGroup::GROUP_NAME,
        bot_id = %inmem_bot,
        environment = %config.environment,
        "agent harness service listening"
    );

    let mut shutdown = std::pin::pin!(shutdown_signal());
    let mut tasks = tokio::task::JoinSet::new();
    let mut run_error = None;
    loop {
        tokio::select! {
            () = &mut shutdown => {
                tracing::info!("agent harness service shutting down");
                match draining_sessions.begin_draining().await {
                    Ok(()) => tracing::info!(%replica, "harness replica is draining"),
                    Err(error) => tracing::error!(
                        error = ?error,
                        %replica,
                        "failed to publish the harness replica drain",
                    ),
                }
                break;
            }
            result = &mut trigger => {
                run_error = Some(match result {
                    Ok(()) => anyhow::anyhow!("agent trigger stopped unexpectedly"),
                    Err(error) => anyhow::anyhow!("agent trigger task failed: {error}"),
                });
                break;
            }
            result = consumer.recv() => {
                let message = match result {
                    Ok(message) => message,
                    Err(error) => {
                        tracing::error!(error = ?error, "failed to receive agent session event");
                        continue;
                    }
                };
                let span = consumer_span(message.inner(), AgentHarnessConsumerGroup::GROUP_NAME);
                let processing = async {
                    let kafka_message = message.inner();
                    let event = match message.decode_payload() {
                        Ok(DeclaredMacroEvent::AgentSessionMacroEvent(event)) => event,
                        Err(error) => {
                            record_span_error(&tracing::Span::current(), &error);
                            tracing::error!(
                                error = ?error,
                                partition = kafka_message.partition(),
                                offset = kafka_message.offset(),
                                "dropping undecodable agent session event"
                            );
                            commit_message(&consumer, kafka_message)?;
                            return Ok::<_, anyhow::Error>(None);
                        }
                    };
                    tracing::Span::current()
                        .record("macro.event.id", tracing::field::display(event.event().event_id));

                    let trigger_event = event.event().event.clone();
                    let runtime = match agent_trigger_bot_id(&trigger_event) {
                        Some(bot_id) => runtime_directory.runtime_for(bot_id).await?,
                        None => None,
                    };
                    let routed = match route_agent_trigger(trigger_event, runtime, &static_file_links) {
                        Ok(routed) => routed,
                        Err(skipped) => {
                            tracing::info!(?skipped, "skipped an agent session event");
                            commit_message(&consumer, kafka_message)?;
                            return Ok(None);
                        }
                    };

                    let RoutedTrigger::Command(session_id, command) = routed;
                    let (session_id, command) = if let HarnessCommand::ConversationMessage(open) = command {
                        let admitted = {
                            let (harness, open) = (&harness, &open);
                            conversation_admission::journal(
                                conversation_admission::ADMISSION_RETRIES,
                                move || harness.admit_conversation_message(session_id, open.clone()),
                            )
                            .await
                        };
                        let admitted = match admitted {
                            Ok(admitted) => admitted,
                            Err(error) => {
                                record_span_error(&tracing::Span::current(), &error);
                                tracing::error!(error = ?error, %session_id, "acknowledging a conversation message the journal could not take");
                                commit_message(&consumer, kafka_message)?;
                                return Ok(None);
                            }
                        };
                        let Some((session_id, open)) = admitted else {
                            tracing::warn!(%session_id, "dropping a conversation message that can no longer be admitted");
                            commit_message(&consumer, kafka_message)?;
                            return Ok(None);
                        };
                        (session_id, HarnessCommand::ConversationMessage(open))
                    } else {
                        (session_id, command)
                    };
                    tracing::Span::current()
                        .record("agent.session.id", tracing::field::display(session_id));
                    let event_type = match &command {
                        HarnessCommand::Open(_) => "agent_trigger.new",
                        HarnessCommand::Deliver(_) => "agent_trigger.existing",
                        HarnessCommand::Delete => "agent_trigger.delete",
                        HarnessCommand::ConversationMessage(_) => "agent_trigger.conversation_message",
                        HarnessCommand::EditQueued { .. }
                        | HarnessCommand::RemoveQueued { .. }
                        | HarnessCommand::SteerQueued { .. }
                        | HarnessCommand::Turn(_)
                        | HarnessCommand::ToolApproval(_)
                        | HarnessCommand::PresentTool { .. }
                        | HarnessCommand::SessionStopped { .. } => "agent_trigger.unexpected",
                    };
                    tracing::Span::current().record("macro.event.type", event_type);
                    let execution_span = tracing::info_span!(
                        "harness.execute",
                        agent.session.id = %session_id,
                        agent.command.type = event_type,
                        otel.status_code = tracing::field::Empty,
                        otel.status_description = tracing::field::Empty,
                    );
                    let execution = execution_span
                        .in_scope(|| harness.execute(session_id, command));
                    let execution = async move { execution.await.map(drop) };
                    let pending = PendingHarnessWork {
                        session_id,
                        span: execution_span,
                        work: Box::pin(execution),
                        description: "executed an agent harness command",
                    };

                    commit_message(&consumer, kafka_message)?;
                    Ok(Some(pending))
                }
                .instrument(span.clone())
                .await;

                let pending = match processing {
                    Ok(pending) => pending,
                    Err(error) => {
                        record_span_error(&span, &error);
                        run_error = Some(error);
                        break;
                    }
                };
                if let Some(PendingHarnessWork { session_id, span, work, description }) = pending {
                    tasks.spawn(async move {
                        match work.instrument(span.clone()).await {
                            Ok(()) => tracing::info!(%session_id, description, "agent harness work completed"),
                            Err(error) => {
                                record_span_error(&span, &error);
                                tracing::error!(error = ?error, %session_id, "agent harness work failed");
                            }
                        }
                    });
                }
            }
            Some(result) = tasks.join_next(), if !tasks.is_empty() => {
                if let Err(error) = result {
                    tracing::error!(error = ?error, "agent harness task failed");
                }
            }
        }
    }

    http.abort();
    trigger.abort();
    egress_http.abort();
    heartbeat.abort();
    dm_recovery.abort();
    typing_heartbeat.abort();
    recovery.abort();
    runtime_commands.abort();
    if let Some(runner) = &code_runner {
        runner.shutdown().await;
    }

    while let Some(result) = tasks.join_next().await {
        if let Err(error) = result {
            tracing::error!(error = ?error, "agent harness task failed during shutdown");
        }
    }

    event_broker_tracker.close();
    if tokio::time::timeout(
        std::time::Duration::from_secs(10),
        event_broker_tracker.wait(),
    )
    .await
    .is_err()
    {
        tracing::warn!("timed out draining in-memory agent event publishes");
    }

    match run_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
