use rewrit_model::ContractInput;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HttpRequestSpec {
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    pub json: Option<serde_json::Value>,
    pub body: Option<String>,
}

impl From<&ContractInput> for HttpRequestSpec {
    fn from(input: &ContractInput) -> Self {
        Self {
            method: input.method.clone().unwrap_or_else(|| "GET".to_string()),
            path: input.path.clone().unwrap_or_else(|| "/".to_string()),
            headers: input.headers.clone(),
            json: input.json.clone(),
            body: input.body.clone(),
        }
    }
}
