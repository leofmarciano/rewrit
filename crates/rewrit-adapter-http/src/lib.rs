//! HTTP adapter primitives for contract-driven parity checks.

#![forbid(unsafe_code)]

pub mod request;
pub mod response;
pub mod server;

pub const ADAPTER_NAME: &str = "http";

use crate::request::HttpRequestSpec;
use rewrit_model::{CanonicalValue, CapturedText, CaseStatus, Contract, Observation, RuntimeId};
use std::collections::BTreeMap;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpAdapterError {
    #[error("invalid base URL {base_url}: {message}")]
    InvalidBaseUrl { base_url: String, message: String },
    #[error("failed to build request URL for {path}: {message}")]
    InvalidRequestUrl { path: String, message: String },
    #[error("healthcheck failed for {url}: {message}")]
    Healthcheck { url: String, message: String },
    #[error("request failed for case {case_id}: {source}")]
    Request {
        case_id: String,
        #[source]
        source: reqwest::Error,
    },
}

pub fn base_url_from_healthcheck(healthcheck: &str) -> Result<String, HttpAdapterError> {
    let url =
        reqwest::Url::parse(healthcheck).map_err(|source| HttpAdapterError::InvalidBaseUrl {
            base_url: healthcheck.to_string(),
            message: source.to_string(),
        })?;
    let origin = url.origin().ascii_serialization();
    Ok(format!("{origin}/"))
}

pub async fn wait_for_healthcheck(
    healthcheck: &str,
    timeout: Duration,
) -> Result<(), HttpAdapterError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|source| HttpAdapterError::Healthcheck {
            url: healthcheck.to_string(),
            message: source.to_string(),
        })?;
    let started = std::time::Instant::now();
    loop {
        match client.get(healthcheck).send().await {
            Ok(response) if response.status().is_success() => return Ok(()),
            _ if started.elapsed() >= timeout => {
                return Err(HttpAdapterError::Healthcheck {
                    url: healthcheck.to_string(),
                    message: format!("not healthy after {}ms", timeout.as_millis()),
                });
            }
            _ => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
}

pub async fn execute_contract(
    base_url: &str,
    runtime_id: RuntimeId,
    contract: &Contract,
    timeout: Duration,
) -> Result<Observation, HttpAdapterError> {
    let base =
        reqwest::Url::parse(base_url).map_err(|source| HttpAdapterError::InvalidBaseUrl {
            base_url: base_url.to_string(),
            message: source.to_string(),
        })?;
    let spec = HttpRequestSpec::from(&contract.input);
    let url = base
        .join(spec.path.trim_start_matches('/'))
        .map_err(|source| HttpAdapterError::InvalidRequestUrl {
            path: spec.path.clone(),
            message: source.to_string(),
        })?;

    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|source| HttpAdapterError::Request {
            case_id: contract.id.to_string(),
            source,
        })?;

    let method = spec
        .method
        .parse::<reqwest::Method>()
        .unwrap_or(reqwest::Method::GET);
    let mut request = client.request(method, url);
    for (key, value) in &spec.headers {
        request = request.header(key, value);
    }
    if let Some(json) = &spec.json {
        request = request.json(json);
    } else if let Some(body) = &spec.body {
        request = request.body(body.clone());
    }

    let started = std::time::Instant::now();
    let response = request
        .send()
        .await
        .map_err(|source| HttpAdapterError::Request {
            case_id: contract.id.to_string(),
            source,
        })?;
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .filter_map(|(key, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (key.as_str().to_ascii_lowercase(), value.to_string()))
        })
        .collect::<BTreeMap<_, _>>();
    let text = response
        .text()
        .await
        .map_err(|source| HttpAdapterError::Request {
            case_id: contract.id.to_string(),
            source,
        })?;
    let body_value = serde_json::from_str::<serde_json::Value>(&text)
        .map(|value| CanonicalValue::Json { value })
        .unwrap_or_else(|_| CanonicalValue::String {
            value: text.clone(),
        });

    Ok(Observation {
        case_id: contract.id.clone(),
        runtime_id,
        status: if status < 500 {
            CaseStatus::Passed
        } else {
            CaseStatus::Failed
        },
        value: Some(http_value(status, headers, body_value)),
        error: None,
        stdout: CapturedText::default(),
        stderr: CapturedText::default(),
        exit_code: Some(0),
        duration_ms: started.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
        effects: Vec::new(),
        artifacts: Vec::new(),
        metadata: BTreeMap::new(),
    })
}

fn http_value(
    status: u16,
    headers: BTreeMap<String, String>,
    body: CanonicalValue,
) -> CanonicalValue {
    CanonicalValue::Object {
        fields: BTreeMap::from([
            (
                "status".to_string(),
                CanonicalValue::Integer {
                    value: status.to_string(),
                },
            ),
            (
                "headers".to_string(),
                CanonicalValue::Object {
                    fields: headers
                        .into_iter()
                        .map(|(key, value)| (key, CanonicalValue::String { value }))
                        .collect(),
                },
            ),
            ("body".to_string(), body),
        ]),
    }
}
