use crate::value::CanonicalValue;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Effect {
    DbDelta(DbDelta),
    FileDelta(FileDelta),
    HttpCall(HttpCall),
    QueueMessage(QueueMessage),
    Event(EventEmission),
    Email(EmailEmission),
    CacheOperation(CacheOperation),
    Log(LogRecord),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DbDelta {
    pub connection: String,
    pub table: String,
    #[serde(default)]
    pub inserted: Vec<BTreeMap<String, CanonicalValue>>,
    #[serde(default)]
    pub updated: Vec<BTreeMap<String, CanonicalValue>>,
    #[serde(default)]
    pub deleted: Vec<BTreeMap<String, CanonicalValue>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DbMap {
    pub target_table: String,
    #[serde(default)]
    pub fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FileDelta {
    pub path: String,
    pub operation: FileOperation,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FileOperation {
    Created,
    Updated,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HttpCall {
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    #[serde(default)]
    pub request_headers: BTreeMap<String, String>,
    #[serde(default)]
    pub response_headers: BTreeMap<String, String>,
    pub request_body: Option<CanonicalValue>,
    pub response_body: Option<CanonicalValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct QueueMessage {
    pub queue: String,
    pub topic: Option<String>,
    pub payload: CanonicalValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EventEmission {
    pub name: String,
    pub payload: CanonicalValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EmailEmission {
    pub to: Vec<String>,
    pub subject: String,
    pub body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CacheOperation {
    pub operation: String,
    pub key: String,
    pub value: Option<CanonicalValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LogRecord {
    pub level: String,
    pub message: String,
    #[serde(default)]
    pub fields: BTreeMap<String, String>,
}

