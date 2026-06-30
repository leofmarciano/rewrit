use crate::version::EVENT_SCHEMA_VERSION;
use rewrit_model::case::Case;
use rewrit_model::ids::{CaseId, RuntimeId};
use rewrit_model::observation::Observation;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DoctorReport {
    pub ok: bool,
    #[serde(default)]
    pub checks: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AdapterEvent {
    DoctorReport {
        schema_version: String,
        runtime_id: RuntimeId,
        report: DoctorReport,
    },
    CaseDiscovered {
        schema_version: String,
        runtime_id: RuntimeId,
        case: Case,
    },
    CaseStarted {
        schema_version: String,
        case_id: CaseId,
        runtime_id: RuntimeId,
    },
    Observation {
        schema_version: String,
        #[serde(flatten)]
        observation: Observation,
    },
    CaseFinished {
        schema_version: String,
        case_id: CaseId,
        runtime_id: RuntimeId,
        duration_ms: u64,
    },
    AdapterError {
        schema_version: String,
        runtime_id: RuntimeId,
        case_id: Option<CaseId>,
        message: String,
        retryable: bool,
    },
}

impl AdapterEvent {
    #[must_use]
    pub fn observation(observation: Observation) -> Self {
        Self::Observation {
            schema_version: EVENT_SCHEMA_VERSION.to_string(),
            observation,
        }
    }
}
