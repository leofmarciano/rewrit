use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerSpec {
    pub start: Vec<String>,
    pub healthcheck: Option<String>,
    pub timeout: Duration,
}

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("server start command is empty")]
    EmptyStartCommand,
    #[error("server healthcheck failed: {0}")]
    Healthcheck(String),
}
