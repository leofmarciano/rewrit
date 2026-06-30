use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub engine: SandboxEngine,
    pub image: Option<String>,
    #[serde(default)]
    pub network: SandboxNetwork,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            engine: SandboxEngine::Docker,
            image: None,
            network: SandboxNetwork::Inherit,
            extra_args: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxEngine {
    #[default]
    Docker,
    Podman,
}

impl SandboxEngine {
    #[must_use]
    pub fn command(self) -> &'static str {
        match self {
            Self::Docker => "docker",
            Self::Podman => "podman",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxNetwork {
    #[default]
    Inherit,
    Disabled,
    Host,
}
