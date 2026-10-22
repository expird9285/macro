use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use agent_client_protocol::schema::v1::{
    SessionConfigOption, SessionConfigSelectOption, SessionConfigValueId,
};

use super::*;

fn caller() -> MacroUserIdStr<'static> {
    MacroUserIdStr::try_from_email("models@example.com").unwrap()
}

fn options() -> Vec<SessionConfigOption> {
    vec![SessionConfigOption::select(
        "model",
        "Model",
        SessionConfigValueId::new("fast"),
        vec![SessionConfigSelectOption::new("fast", "Fast")],
    )]
}

struct Probe {
    callers: std::sync::Mutex<Vec<MacroUserIdStr<'static>>>,
    calls: AtomicUsize,
    result: fn() -> Result<RawModelProbe, ModelProbeError>,
}

impl Probe {
    fn new(result: fn() -> Result<RawModelProbe, ModelProbeError>) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            callers: std::sync::Mutex::new(Vec::new()),
            result,
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }
}

impl InMemoryModelProbe for Probe {
    async fn probe(
        &self,
        caller: &MacroUserIdStr<'static>,
    ) -> Result<RawModelProbe, ModelProbeError> {
        self.callers.lock().unwrap().push(caller.clone());
        self.calls.fetch_add(1, Ordering::Relaxed);
        (self.result)()
    }
}

fn available() -> Result<RawModelProbe, ModelProbeError> {
    Ok(RawModelProbe::Options(options()))
}

fn unsupported() -> Result<RawModelProbe, ModelProbeError> {
    Ok(RawModelProbe::Unsupported)
}

struct HangingProbe;

impl InMemoryModelProbe for HangingProbe {
    async fn probe(
        &self,
        _caller: &MacroUserIdStr<'static>,
    ) -> Result<RawModelProbe, ModelProbeError> {
        std::future::pending().await
    }
}

#[tokio::test]
async fn in_memory_returns_advertised_catalog() {
    let service = AgentModelsServiceImpl::new(
        Probe::new(available),
        Duration::from_secs(1),
    );
    let result = service
        .load(
            caller(),
            LoadAgentModels {
                harness: ModelHarness::InMemory,
            },
        )
        .await
        .unwrap();

    assert_eq!(result.status, AgentModelsStatus::Available);
    assert_eq!(result.current_model.as_deref(), Some("fast"));
    assert_eq!(result.models.len(), 1);
    assert_eq!(result.models[0].id, "fast");
    assert_eq!(service.in_memory.calls(), 1);
    assert_eq!(*service.in_memory.callers.lock().unwrap(), vec![caller()]);
}

#[tokio::test]
async fn unsupported_provider_returns_the_supported_response_shape() {
    let service = AgentModelsServiceImpl::new(
        Probe::new(unsupported),
        Duration::from_secs(1),
    );

    let result = service
        .load(
            caller(),
            LoadAgentModels {
                harness: ModelHarness::InMemory,
            },
        )
        .await
        .unwrap();

    assert_eq!(result, AgentModels::unsupported());
    assert_eq!(service.in_memory.calls(), 1);
    assert_eq!(*service.in_memory.callers.lock().unwrap(), vec![caller()]);
}

#[tokio::test]
async fn in_memory_probe_obeys_the_service_timeout() {
    let service = AgentModelsServiceImpl::new(
        HangingProbe,
        Duration::from_millis(1),
    );

    let result = service
        .load(
            caller(),
            LoadAgentModels {
                harness: ModelHarness::InMemory,
            },
        )
        .await;

    assert!(matches!(result, Err(LoadAgentModelsError::Timeout)));
}
