use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CanonicalValue {
    Null,
    Absent,
    Bool {
        value: bool,
    },
    Integer {
        value: String,
    },
    Decimal {
        value: String,
    },
    Float {
        value: String,
    },
    String {
        value: String,
    },
    Bytes {
        base64: String,
        media_type: Option<String>,
    },
    Array {
        items: Vec<CanonicalValue>,
    },
    Object {
        fields: BTreeMap<String, CanonicalValue>,
    },
    DateTime {
        rfc3339: String,
    },
    Json {
        value: Value,
    },
}

impl CanonicalValue {
    #[must_use]
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Absent => "absent",
            Self::Bool { .. } => "bool",
            Self::Integer { .. } => "integer",
            Self::Decimal { .. } => "decimal",
            Self::Float { .. } => "float",
            Self::String { .. } => "string",
            Self::Bytes { .. } => "bytes",
            Self::Array { .. } => "array",
            Self::Object { .. } => "object",
            Self::DateTime { .. } => "date_time",
            Self::Json { .. } => "json",
        }
    }
}
