//! Redis command-bus listener composed around the harness domain service.

use std::sync::Arc;

use agent_harness::domain::service::ForwardedCommands;
use agent_harness::outbound::forward::{
    COMMAND_CHANNEL, RuntimeCommandRequest, RuntimeCommandTarget,
};
use agent_session::domain::model::ReplicaId;
use futures::StreamExt as _;
use tracing::Instrument as _;

#[cfg(test)]
mod test;

pub(crate) async fn consume_runtime_commands<Harness>(
    redis: redis::Client,
    replica: ReplicaId,
    harness_service: Arc<Harness>,
    ready: tokio::sync::watch::Sender<bool>,
) -> anyhow::Result<()>
where
    Harness: ForwardedCommands,
{
    let mut subscriber = redis.get_async_pubsub().await?;
    subscriber.subscribe(COMMAND_CHANNEL).await?;
    ready.send_replace(true);
    let mut requests = subscriber.into_on_message();
    while let Some(request) = requests.next().await {
        let payload: String = match request.get_payload() {
            Ok(payload) => payload,
            Err(error) => {
                tracing::warn!(error = ?error, "dropping malformed runtime command broadcast");
                continue;
            }
        };
        let request: RuntimeCommandRequest = match serde_json::from_str(&payload) {
            Ok(request) => request,
            Err(error) => {
                tracing::warn!(error = ?error, "dropping malformed runtime command broadcast");
                continue;
            }
        };
        let harness_service = Arc::clone(&harness_service);
        let span = request.processing_span();
        tokio::spawn(
            async move {
                let session = request.session();
                let RuntimeCommandTarget::Replica(target) = request.target();
                if target == replica.as_uuid() {
                    match harness_service
                        .execute_forwarded(session, request.into_command())
                        .await
                    {
                        Ok(outcome) => {
                            tracing::info!(%session, ?outcome, "executed a forwarded command");
                        }
                        Err(error) => {
                            tracing::error!(error = ?error, %session, "forwarded command failed");
                        }
                    }
                }
            }
            .instrument(span),
        );
    }
    anyhow::bail!("runtime command Redis subscription ended")
}
