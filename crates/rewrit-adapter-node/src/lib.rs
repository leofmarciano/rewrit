//! Node adapter boundary for Vitest, Jest and Encore helpers.

#![forbid(unsafe_code)]

pub const VITEST_ADAPTER: &str = "node:vitest";
pub const JEST_ADAPTER: &str = "node:jest";
pub const ENCORE_ADAPTER: &str = "node:encore";

pub mod encore {
    pub const HELPER: &str = "@rewrit/node/encore";
}

pub mod jest {
    pub const REPORTER: &str = "@rewrit/node/jest-reporter";
}

pub mod vitest {
    pub const REPORTER: &str = "@rewrit/node/vitest-reporter";
}
