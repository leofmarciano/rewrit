use crate::version::ADAPTER_REQUEST_SCHEMA_VERSION;
use rewrit_model::ids::{CaseId, RuntimeId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdapterCommand {
    Doctor,
    Discover,
    Run,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AdapterRequest {
    pub schema_version: String,
    pub command: AdapterCommand,
    pub runtime_id: RuntimeId,
    #[serde(default)]
    pub cases: Vec<CaseId>,
}

impl AdapterRequest {
    #[must_use]
    pub fn new(command: AdapterCommand, runtime_id: RuntimeId, cases: Vec<CaseId>) -> Self {
        Self {
            schema_version: ADAPTER_REQUEST_SCHEMA_VERSION.to_string(),
            command,
            runtime_id,
            cases,
        }
    }
}
