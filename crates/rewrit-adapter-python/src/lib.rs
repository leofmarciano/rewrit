//! Python adapter boundary for pytest and Django helpers.

#![forbid(unsafe_code)]

pub const PYTEST_ADAPTER: &str = "python:pytest";
pub const DJANGO_ADAPTER: &str = "python:django";

pub mod django {
    pub const HELPER: &str = "rewrit_pytest.django";
    pub const DB_DELTA_HELPER: &str = "rewrit_pytest.django.observe_db_delta";
    pub const HTTP_RESPONSE_HELPER: &str = "rewrit_pytest.django.observe_http_response";
}

pub mod pytest {
    pub const PLUGIN: &str = "rewrit_pytest.plugin";
    pub const CASE_DECORATOR: &str = "rewrit_pytest.rewrit_case";
}
