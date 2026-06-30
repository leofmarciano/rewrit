use crate::case::SourceLocation;
use crate::ids::CaseId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Blocking,
    Warning,
    Allowed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DivergenceKind {
    MissingCandidateCase,
    MissingReferenceCase,
    OrphanCandidateCase,
    OutputMismatch,
    TypeMismatch,
    SchemaMismatch,
    ErrorMismatch,
    SideEffectMismatch,
    StdoutMismatch,
    StderrMismatch,
    ExitCodeMismatch,
    Timeout,
    Flaky,
    AdapterError,
    InfraError,
    PolicyAllowed,
    WaiverExpired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MinimalReproduction {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Divergence {
    pub kind: DivergenceKind,
    pub severity: Severity,
    pub case_id: CaseId,
    pub suite: Option<String>,
    pub path: Option<String>,
    pub reference: Option<Value>,
    pub candidate: Option<Value>,
    pub message: String,
    pub machine_code: String,
    pub source_location: Option<SourceLocation>,
    pub target_location: Option<SourceLocation>,
    pub policy: Option<String>,
    #[serde(default)]
    pub normalizers_applied: Vec<String>,
    pub hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimal_reproduction: Option<MinimalReproduction>,
}
