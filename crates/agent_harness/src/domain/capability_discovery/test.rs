use std::sync::Mutex;
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

struct Probe<Target> {
    targets: Mutex<Vec<Target>>,
    result: fn() -> Result<RawCapabilityProbe, CapabilityProbeError>,
}

impl<Target> Probe<Target> {
    fn new(result: fn() -> Result<RawCapabilityProbe, CapabilityProbeError>) -> Self {
        Self {
            targets: Mutex::new(Vec::new()),
            result,
        }
    }

    fn calls(&self) -> usize {
        self.targets.lock().unwrap().len()
    }
}

impl<Target: Clone + Send + Sync + 'static> CapabilityProbe for Probe<Target> {
    type Target = Target;

    async fn probe(
        &self,
        target: &Target,
        _model: Option<&str>,
    ) -> Result<RawCapabilityProbe, CapabilityProbeError> {
        self.targets.lock().unwrap().push(target.clone());
        (self.result)()
    }
}

fn available() -> Result<RawCapabilityProbe, CapabilityProbeError> {
    Ok(RawCapabilityProbe::Options(options()))
}

#[tokio::test]
async fn in_memory_dispatches_and_projects_the_catalog() {
    let in_memory = Probe::new(available);
    let service = AgentCapabilitiesServiceImpl::new(
        in_memory,
        Duration::from_secs(1),
    );

    let capabilities = service
        .load(
            caller(),
            DiscoverAgentCapabilities {
                harness: CapabilityHarness::InMemory,
                model: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(capabilities.config_options.len(), 1);
    assert_eq!(capabilities.config_options[0].id.as_str(), "model");
}
