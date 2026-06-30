use rewrit_model::CanonicalValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HttpResponseObservation {
    pub status: u16,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    pub body: Option<CanonicalValue>,
}

