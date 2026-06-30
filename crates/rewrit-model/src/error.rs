use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CanonicalError {
    pub kind: ErrorKind,
    pub code: Option<String>,
    pub class: Option<String>,
    pub message: Option<String>,
    pub normalized_message: Option<String>,
    pub http_status: Option<u16>,
    pub retryable: Option<bool>,
    #[serde(default)]
    pub frames: Vec<StackFrame>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ErrorKind {
    Exception,
    Panic,
    Validation,
    Authorization,
    NotFound,
    Conflict,
    Timeout,
    ProcessExit,
    AssertionFailure,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StackFrame {
    pub function: Option<String>,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
}
