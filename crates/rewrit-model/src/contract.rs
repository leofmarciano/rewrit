use crate::ids::CaseId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Contract {
    pub schema_version: String,
    pub id: CaseId,
    pub kind: String,
    #[serde(default)]
    pub input: ContractInput,
    #[serde(default)]
    pub expect: ContractExpectation,
    pub policy: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ContractInput {
    pub method: Option<String>,
    pub path: Option<String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    pub json: Option<Value>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ContractExpectation {
    pub status: Option<u16>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    pub json: Option<Value>,
    pub json_schema: Option<Value>,
    #[serde(default)]
    pub effects: Vec<Value>,
}

