//! Rust adapter boundary for cargo test and Rust SDK helpers.

#![forbid(unsafe_code)]

pub const CARGO_TEST_ADAPTER: &str = "rust:cargo_test";

pub mod cargo_test {
    pub const ENV_FLAG: &str = "REWRIT_PROTOCOL";
}

