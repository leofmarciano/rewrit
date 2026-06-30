use crate::effect::Effect;
use crate::error::CanonicalError;
use crate::ids::{CaseId, RuntimeId};
use crate::value::CanonicalValue;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CaseStatus {
    Passed,
    Failed,
    Skipped,
    TimedOut,
    AdapterError,
    InfraError,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CapturedText {
    pub text: String,
    #[serde(default)]
    pub truncated: bool,
}

impl CapturedText {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            truncated: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Artifact {
    pub path: String,
    pub media_type: Option<String>,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Observation {
    pub case_id: CaseId,
    pub runtime_id: RuntimeId,
    pub status: CaseStatus,
    pub value: Option<CanonicalValue>,
    pub error: Option<CanonicalError>,
    #[serde(default)]
    pub stdout: CapturedText,
    #[serde(default)]
    pub stderr: CapturedText,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    #[serde(default)]
    pub effects: Vec<Effect>,
    #[serde(default)]
    pub artifacts: Vec<Artifact>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}
