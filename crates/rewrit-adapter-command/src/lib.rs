//! Generic command adapter.
//!
//! This adapter runs any process that emits the Rewrit NDJSON protocol. The
//! actual process execution lives in `rewrit-engine`; this crate marks the
//! adapter boundary and keeps the public adapter name stable.

#![forbid(unsafe_code)]

pub const ADAPTER_NAME: &str = "command";

#[must_use]
pub fn is_command_adapter(name: &str) -> bool {
    name == ADAPTER_NAME || name.starts_with("command:")
}
