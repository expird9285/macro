//! Domain vocabulary for fresh ACP capability discovery.

use std::future::Future;
use std::time::Duration;

use agent_client_protocol::schema::v1::SessionConfigOption as AcpSessionConfigOption;
use agent_fold::domain::session_config::{SessionConfigOption, session_config_options};
use macro_user_id::user_id::MacroUserIdStr;

#[cfg(test)]
mod test;

/// Provider selected by a capability-discovery request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityHarness {
    /// Macro's in-process agent.
    InMemory,
}

/// One fresh capability-discovery request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoverAgentCapabilities {
    /// Provider to inspect.
    pub harness: CapabilityHarness,
    /// Selected model, when inspecting a preflight override.
    pub model: Option<String>,
}

/// Agent-advertised settings returned by the use case.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilities {
    /// Complete ordered ACP session configuration advertised by the agent.
    pub config_options: Vec<SessionConfigOption>,
}

impl AgentCapabilities {
    fn from_options(options: &[AcpSessionConfigOption]) -> Self {
        Self {
            config_options: session_config_options(options),
        }
    }

    /// A provider with no discoverable session configuration.
    #[must_use]
    pub fn unsupported() -> Self {
        Self {
            config_options: Vec::new(),
        }
    }
}

/// Raw outcome from a provider probe.
#[derive(Debug)]
pub enum RawCapabilityProbe {
    /// ACP session configuration advertised by the provider.
    Options(Vec<AcpSessionConfigOption>),
    /// This provider cannot advertise session configuration.
    Unsupported,
}

/// A provider probe failure.
#[derive(Debug, thiserror::Error)]
pub enum CapabilityProbeError {
    /// A required live runtime is not connected.
    #[error("the requested harness is disconnected")]
    Disconnected,
    /// Provider-specific probing failed.
    #[error("capability probe failed: {0}")]
    Failed(String),
}

/// Use-case failure.
#[derive(Debug, thiserror::Error)]
pub enum DiscoverAgentCapabilitiesError {
    /// The target shape is invalid.
    #[error("{0}")]
    BadRequest(String),
    /// The caller cannot use or see the target harness.
    #[error("forbidden")]
    Forbidden,
    /// A required runtime is disconnected.
    #[error("the requested harness is disconnected")]
    Disconnected,
    /// The bounded request expired.
    #[error("capability probe timed out")]
    Timeout,
    /// A provider failed.
    #[error("capability probe failed: {0}")]
    Probe(String),
}

/// Discovers a harness's advertised configuration without persisting a session.
pub trait CapabilityProbe: Send + Sync + 'static {
    /// Identity needed by the adapter: a caller or `()`.
    type Target: Sync;

    /// Probe fresh capabilities for a target authorized by the domain service.
    fn probe(
        &self,
        target: &Self::Target,
        model: Option<&str>,
    ) -> impl Future<Output = Result<RawCapabilityProbe, CapabilityProbeError>> + Send;
}

/// Authenticated agent-capability discovery use case.
pub trait AgentCapabilitiesService: Send + Sync + 'static {
    /// Discover one target's session settings without creating a persisted session.
    fn load(
        &self,
        caller: MacroUserIdStr<'static>,
        request: DiscoverAgentCapabilities,
    ) -> impl Future<Output = Result<AgentCapabilities, DiscoverAgentCapabilitiesError>> + Send;
}

/// Domain service coordinating authorization, dispatch, timeout, and projection.
pub struct AgentCapabilitiesServiceImpl<InMemory> {
    in_memory: InMemory,
    timeout: Duration,
}

impl<InMemory> AgentCapabilitiesServiceImpl<InMemory> {
    /// Build the service from its outbound ports.
    pub fn new(in_memory: InMemory, timeout: Duration) -> Self {
        Self { in_memory, timeout }
    }
}

impl<InMemory> AgentCapabilitiesService for AgentCapabilitiesServiceImpl<InMemory>
where
    InMemory: CapabilityProbe<Target = MacroUserIdStr<'static>>,
{
    async fn load(
        &self,
        caller: MacroUserIdStr<'static>,
        request: DiscoverAgentCapabilities,
    ) -> Result<AgentCapabilities, DiscoverAgentCapabilitiesError> {
        match request.harness {
            CapabilityHarness::InMemory => {
                self.discover(&self.in_memory, &caller, request.model.as_deref())
                    .await
            }
        }
    }
}

impl<InMemory> AgentCapabilitiesServiceImpl<InMemory> {
    async fn discover<Probe: CapabilityProbe>(
        &self,
        adapter: &Probe,
        target: &Probe::Target,
        model: Option<&str>,
    ) -> Result<AgentCapabilities, DiscoverAgentCapabilitiesError> {
        let probe = tokio::time::timeout(self.timeout, adapter.probe(target, model))
            .await
            .map_err(|_| DiscoverAgentCapabilitiesError::Timeout)?;
        match probe {
            Ok(RawCapabilityProbe::Options(options)) => {
                Ok(AgentCapabilities::from_options(&options))
            }
            Ok(RawCapabilityProbe::Unsupported) => Ok(AgentCapabilities::unsupported()),
            Err(CapabilityProbeError::Disconnected) => {
                Err(DiscoverAgentCapabilitiesError::Disconnected)
            }
            Err(CapabilityProbeError::Failed(message)) => {
                Err(DiscoverAgentCapabilitiesError::Probe(message))
            }
        }
    }
}
