use super::*;
use agent_harness::domain::error::{HarnessError, Result};
use agent_harness::domain::model::{CommandOutcome, HarnessCommand};
use agent_harness::domain::ports::{CommandForwarder as _, CommandTarget};
use agent_harness::outbound::forward::RedisCommandForwarder;
use agent_session::domain::model::AgentSessionId;
use std::sync::atomic::{AtomicUsize, Ordering};

struct RecordingHarness {
    calls: AtomicUsize,
    fails: bool,
}

impl ForwardedCommands for RecordingHarness {
    async fn execute_forwarded(
        &self,
        session_id: AgentSessionId,
        _command: HarnessCommand,
    ) -> Result<CommandOutcome> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fails {
            return Err(HarnessError::Disconnected(session_id));
        }
        Ok(CommandOutcome::Completed)
    }
}

fn redis_client() -> redis::Client {
    let url = macro_env_var::optional_read_env_var("REDIS_URI")
        .ok()
        .flatten()
        .unwrap_or_else(|| "redis://127.0.0.1:6379".to_owned());
    redis::Client::open(url).expect("valid Redis URL")
}

async fn ready(readiness: &mut tokio::sync::watch::Receiver<bool>) {
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        readiness.wait_for(|ready| *ready),
    )
    .await
    .expect("consumer becomes ready")
    .expect("readiness channel remains open");
}

async fn wait_for_calls(harness: &RecordingHarness, expected: usize) {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while harness.calls.load(Ordering::SeqCst) != expected {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("command is processed");
}

fn harness(fails: bool) -> Arc<RecordingHarness> {
    Arc::new(RecordingHarness {
        calls: AtomicUsize::new(0),
        fails,
    })
}

#[tokio::test]
async fn a_command_executes_on_exactly_one_responsible_replica() {
    let redis = redis_client();
    let origin = ReplicaId::mint();
    let peer = ReplicaId::mint();
    let origin_harness = harness(false);
    let peer_harness = harness(false);
    let (origin_ready, mut origin_readiness) = tokio::sync::watch::channel(false);
    let origin_consumer = tokio::spawn(consume_runtime_commands(
        redis.clone(),
        origin,
        Arc::clone(&origin_harness),
        origin_ready,
    ));
    let (peer_ready, mut peer_readiness) = tokio::sync::watch::channel(false);
    let peer_consumer = tokio::spawn(consume_runtime_commands(
        redis.clone(),
        peer,
        Arc::clone(&peer_harness),
        peer_ready,
    ));
    ready(&mut origin_readiness).await;
    ready(&mut peer_readiness).await;

    let outcome = RedisCommandForwarder::new(redis)
        .forward(
            AgentSessionId::TEST_A,
            HarnessCommand::Delete,
            CommandTarget::Replica(peer),
        )
        .await
        .unwrap();

    assert_eq!(outcome, CommandOutcome::Completed);
    wait_for_calls(&peer_harness, 1).await;
    assert_eq!(origin_harness.calls.load(Ordering::SeqCst), 0);
    origin_consumer.abort();
    peer_consumer.abort();
}

#[tokio::test]
async fn a_responsible_replicas_error_does_not_fail_the_publish() {
    let redis = redis_client();
    let replica = ReplicaId::mint();
    let harness = harness(true);
    let (ready_tx, mut readiness) = tokio::sync::watch::channel(false);
    let consumer = tokio::spawn(consume_runtime_commands(
        redis.clone(),
        replica,
        Arc::clone(&harness),
        ready_tx,
    ));
    ready(&mut readiness).await;

    let outcome = RedisCommandForwarder::new(redis)
        .forward(
            AgentSessionId::TEST_A,
            HarnessCommand::Delete,
            CommandTarget::Replica(replica),
        )
        .await
        .expect("the command publishes");

    assert_eq!(outcome, CommandOutcome::Completed);
    wait_for_calls(&harness, 1).await;
    consumer.abort();
}
