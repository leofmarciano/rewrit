//! Python adapter boundary for pytest and Django helpers.

#![forbid(unsafe_code)]

pub const PYTEST_ADAPTER: &str = "python:pytest";
pub const DJANGO_ADAPTER: &str = "python:django";

pub mod django {
    pub const HELPER: &str = "rewrit_pytest.django";
}

pub mod pytest {
    pub const PLUGIN: &str = "rewrit_pytest.plugin";
}

