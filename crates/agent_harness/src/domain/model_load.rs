//! Domain vocabulary for fresh agent-model discovery.

use std::future::Future;
use std::time::Duration;

use agent_client_protocol::schema::v1::SessionConfigOption;
use agent_fold::domain::model::ModelOption;
use agent_fold::domain::model_selection::model_selection;
use macro_user_id::user_id::MacroUserIdStr;

#[cfg(test)]
mod test;

/// Provider selected by a model-discovery request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelHarness {
    /// Macro's in-process agent.
    InMemory,
}

/// One fresh model-discovery request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadAgentModels {
    /// Provider to inspect.
    pub harness: ModelHarness,
}

/// Availability of the requested provider's model picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentModelsStatus {
    /// A model select was advertised.
    Available,
    /// The provider does not advertise a model select.
    Unsupported,
}

/// Model picker data returned by the use case.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModels {
    /// Whether model selection is available.
    pub status: AgentModelsStatus,
    /// Currently selected model, absent when unsupported.
    pub current_model: Option<String>,
    /// Ordered model choices.
    pub models: Vec<ModelOption>,
}

impl AgentModels {
    fn from_options(options: &[SessionConfigOption]) -> Self {
        match model_selection(options) {
            Some(selection) => Self {
                status: AgentModelsStatus::Available,
                current_model: Some(selection.current),
                models: selection.options,
            },
            None => Self::unsupported(),
        }
    }

    /// A provider with no model-selection capability.
    #[must_use]
    pub fn unsupported() -> Self {
        Self {
            status: AgentModelsStatus::Unsupported,
            current_model: None,
            models: Vec::new(),
        }
    }
}

/// Raw outcome from a provider probe.
#[derive(Debug)]
pub enum RawModelProbe {
    /// ACP session configuration advertised by the provider.
    Options(Vec<SessionConfigOption>),
    /// This provider cannot advertise models.
    Unsupported,
}

/// A provider probe failure.
#[derive(Debug, thiserror::Error)]
pub enum ModelProbeError {
    /// A required live runtime is not connected.
    #[error("the requested harness is disconnected")]
    Disconnected,
    /// Provider-specific probing failed.
    #[error("model probe failed: {0}")]
    Failed(String),
}

/// Use-case failure.
#[derive(Debug, thiserror::Error)]
pub enum LoadAgentModelsError {
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
    #[error("model probe timed out")]
    Timeout,
    /// A provider failed.
    #[error("model probe failed: {0}")]
    Probe(String),
}

/// Fresh in-memory model probe.
pub trait InMemoryModelProbe: Send + Sync + 'static {
    /// Probe the running in-memory implementation.
    fn probe(
        &self,
        caller: &MacroUserIdStr<'static>,
    ) -> impl Future<Output = Result<RawModelProbe, ModelProbeError>> + Send;
}

/// Authenticated agent-model discovery use case.
pub trait AgentModelsService: Send + Sync + 'static {
    /// Load one target's model catalog without creating a persisted session.
    fn load(
        &self,
        caller: MacroUserIdStr<'static>,
        request: LoadAgentModels,
    ) -> impl Future<Output = Result<AgentModels, LoadAgentModelsError>> + Send;
}

/// Domain service coordinating authorization, dispatch, timeout, and projection.
pub struct AgentModelsServiceImpl<InMemory> {
    in_memory: InMemory,
    timeout: Duration,
}

impl<InMemory> AgentModelsServiceImpl<InMemory> {
    /// Build the service from its outbound ports.
    pub fn new(in_memory: InMemory, timeout: Duration) -> Self {
        Self { in_memory, timeout }
    }
}

impl<InMemory> AgentModelsService for AgentModelsServiceImpl<InMemory>
where
    InMemory: InMemoryModelProbe,
{
    async fn load(
        &self,
        caller: MacroUserIdStr<'static>,
        request: LoadAgentModels,
    ) -> Result<AgentModels, LoadAgentModelsError> {
        let probe = match request.harness {
            ModelHarness::InMemory => {
                tokio::time::timeout(self.timeout, self.in_memory.probe(&caller))
                    .await
                    .map_err(|_| LoadAgentModelsError::Timeout)?
            }
        };

        match probe {
            Ok(RawModelProbe::Options(options)) => Ok(AgentModels::from_options(&options)),
            Ok(RawModelProbe::Unsupported) => Ok(AgentModels::unsupported()),
            Err(ModelProbeError::Disconnected) => Err(LoadAgentModelsError::Disconnected),
            Err(ModelProbeError::Failed(message)) => Err(LoadAgentModelsError::Probe(message)),
        }
    }
}
