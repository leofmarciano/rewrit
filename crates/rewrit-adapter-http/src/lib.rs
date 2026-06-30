//! HTTP adapter primitives for contract-driven parity checks.

#![forbid(unsafe_code)]

pub mod request;
pub mod response;
pub mod server;

pub const ADAPTER_NAME: &str = "http";
