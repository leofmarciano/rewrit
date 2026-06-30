//! Rust adapter boundary for cargo test and Rust SDK helpers.

#![forbid(unsafe_code)]

pub const CARGO_TEST_ADAPTER: &str = "rust:cargo_test";

pub mod cargo_test {
    pub const DEFAULT_COMMAND: [&str; 4] = ["cargo", "test", "--", "--nocapture"];
    pub const ENV_FLAG: &str = "REWRIT_PROTOCOL";
    pub const EXPLICIT_HELPER: &str = "rewrit::cargo_test_case";
    pub const CASE_MACRO: &str = "#[rewrit::case]";
    pub const OBSERVE_JSON_HELPER: &str = "rewrit::observe_json";
}
