use crate::divergence::Divergence;
use crate::ids::CaseId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Report {
    pub schema_version: String,
    pub run_id: String,
    pub project: String,
    pub reference: String,
    pub candidate: String,
    pub summary: ReportSummary,
    #[serde(default)]
    pub suites: Vec<SuiteSummary>,
    #[serde(default)]
    pub divergences: Vec<Divergence>,
    #[serde(default)]
    pub normalizers_applied: Vec<AppliedNormalizer>,
    #[serde(default)]
    pub policy_trace: Vec<PolicyDecision>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ReportSummary {
    pub cases_discovered: usize,
    pub cases_compared: usize,
    pub equivalent: usize,
    pub waived: usize,
    pub blocking: usize,
    pub warnings: usize,
    pub parity_ratio: f64,
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SuiteSummary {
    pub suite_id: String,
    pub cases_compared: usize,
    pub equivalent: usize,
    pub blocking: usize,
    pub parity_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AppliedNormalizer {
    pub case_id: CaseId,
    pub name: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PolicyDecision {
    pub case_id: CaseId,
    pub policy: String,
    pub decision: String,
    pub reason: String,
}
