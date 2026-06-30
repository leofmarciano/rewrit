//! Report rendering for Rewrit runs.

#![forbid(unsafe_code)]

pub mod html;
pub mod json;
pub mod junit;
pub mod markdown;
pub mod ndjson;
pub mod sarif;
pub mod terminal;

use rewrit_model::Report;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReportError {
    #[error("unsupported report kind: {0}")]
    UnsupportedKind(String),
    #[error("failed to serialize report: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("failed to write report {path}: {source}")]
    Write {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

pub fn render(kind: &str, report: &Report) -> Result<String, ReportError> {
    match kind {
        "terminal" => Ok(terminal::render(report)),
        "json" => json::render(report).map_err(ReportError::from),
        "ndjson" => ndjson::render(report).map_err(ReportError::from),
        "junit" => Ok(junit::render(report)),
        "sarif" => sarif::render(report).map_err(ReportError::from),
        "html" => Ok(html::render(report)),
        "markdown" => Ok(markdown::render(report)),
        other => Err(ReportError::UnsupportedKind(other.to_string())),
    }
}

pub fn write(kind: &str, path: impl AsRef<Path>, report: &Report) -> Result<(), ReportError> {
    let path = path.as_ref();
    let rendered = render(kind, report)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| ReportError::Write {
            path: parent.display().to_string(),
            source,
        })?;
    }
    std::fs::write(path, rendered).map_err(|source| ReportError::Write {
        path: path.display().to_string(),
        source,
    })
}

pub(crate) fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

