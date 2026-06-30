use crate::discovery::binding::BindingConfig;
use rewrit_model::{RuntimeId, SuiteId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub project: ProjectConfig,
    #[serde(default)]
    pub runtimes: BTreeMap<RuntimeId, RuntimeConfig>,
    #[serde(default)]
    pub suites: Vec<SuiteConfig>,
    #[serde(default)]
    pub bindings: Vec<BindingConfig>,
    #[serde(default)]
    pub policies: BTreeMap<String, PolicyConfig>,
    #[serde(default)]
    pub normalizers: Vec<NormalizerConfig>,
    #[serde(default)]
    pub reports: Vec<ReportConfig>,
    #[serde(default)]
    pub waivers: Vec<WaiverConfig>,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub runner: RunnerConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub reference: RuntimeId,
    pub candidate: RuntimeId,
    pub contracts_dir: Option<String>,
    pub baselines_dir: Option<String>,
    pub reports_dir: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub adapter: String,
    pub cwd: Option<String>,
    #[serde(default)]
    pub command: Vec<String>,
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    pub server: Option<ServerConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default)]
    pub start: Vec<String>,
    pub healthcheck: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuiteConfig {
    pub id: SuiteId,
    pub title: Option<String>,
    pub source_glob: Option<String>,
    pub policy: Option<String>,
    #[serde(default = "default_true")]
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub mode: Option<String>,
    pub allow_missing_candidate: Option<bool>,
    pub allow_extra_candidate: Option<bool>,
    pub fail_on_orphan_candidate: Option<bool>,
    pub compare_status: Option<bool>,
    pub compare_json: Option<bool>,
    pub compare_headers: Option<bool>,
    pub compare_effects: Option<bool>,
    pub compare_stdout: Option<bool>,
    pub compare_stderr: Option<bool>,
    pub compare_duration: Option<bool>,
    pub compare_exit_code: Option<bool>,
    pub numeric_epsilon: Option<String>,
    pub allow_integer_float_equivalence: Option<bool>,
    pub allow_header_case_difference: Option<bool>,
    pub allow_object_key_order_difference: Option<bool>,
    pub allow_null_absent_equivalence: Option<bool>,
    pub decimal_as_string: Option<bool>,
    pub ignore_stack_trace: Option<bool>,
    pub headers: Option<HeadersPolicyConfig>,
    pub json: Option<JsonPolicyConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadersPolicyConfig {
    #[serde(default)]
    pub ignore: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonPolicyConfig {
    #[serde(default)]
    pub unordered_paths: Vec<String>,
    #[serde(default)]
    pub ignore_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizerConfig {
    pub kind: String,
    pub pattern: Option<String>,
    pub replacement: Option<String>,
    pub replace_project_root: Option<String>,
    #[serde(default)]
    pub paths: Vec<String>,
    pub lowercase_names: Option<bool>,
    pub sort_values: Option<bool>,
    pub detect_lists: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportConfig {
    pub kind: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaiverConfig {
    pub case: rewrit_model::CaseId,
    pub kind: rewrit_model::DivergenceKind,
    pub reason: String,
    pub owner: String,
    pub expires: String,
    pub issue: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default = "default_true")]
    pub redact_env: bool,
    #[serde(default)]
    pub redact_patterns: Vec<String>,
    #[serde(default)]
    pub env_allowlist: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            redact_env: true,
            redact_patterns: vec![
                "sk_live_[A-Za-z0-9]+".to_string(),
                "Bearer [A-Za-z0-9._-]+".to_string(),
            ],
            env_allowlist: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunnerConfig {
    #[serde(default = "default_true")]
    pub kill_process_tree: bool,
    pub default_timeout_ms: Option<u64>,
    pub max_stdout_bytes: Option<usize>,
    pub max_stderr_bytes: Option<usize>,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            kill_process_tree: true,
            default_timeout_ms: Some(30_000),
            max_stdout_bytes: Some(1_048_576),
            max_stderr_bytes: Some(1_048_576),
        }
    }
}

fn default_true() -> bool {
    true
}
