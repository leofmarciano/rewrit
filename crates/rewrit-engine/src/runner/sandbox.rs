#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxConfig {
    pub enabled: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self { enabled: false }
    }
}

