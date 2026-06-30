use crate::events::AdapterEvent;
use crate::version::EVENT_SCHEMA_VERSION;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("invalid JSON event at line {line}: {source}")]
    InvalidJson {
        line: usize,
        #[source]
        source: serde_json::Error,
    },
    #[error("unsupported schema_version at line {line}: {schema_version}")]
    UnsupportedVersion { line: usize, schema_version: String },
    #[error("event missing schema_version at line {line}")]
    MissingVersion { line: usize },
}

pub fn encode_event_line(event: &AdapterEvent) -> Result<String, serde_json::Error> {
    let mut encoded = serde_json::to_string(event)?;
    encoded.push('\n');
    Ok(encoded)
}

pub fn decode_event_line(line: &str, line_number: usize) -> Result<Option<AdapterEvent>, ProtocolError> {
    if line.trim().is_empty() {
        return Ok(None);
    }

    let value: serde_json::Value =
        serde_json::from_str(line).map_err(|source| ProtocolError::InvalidJson {
            line: line_number,
            source,
        })?;

    let Some(schema_version) = value.get("schema_version").and_then(serde_json::Value::as_str) else {
        return Err(ProtocolError::MissingVersion { line: line_number });
    };

    if schema_version != EVENT_SCHEMA_VERSION {
        return Err(ProtocolError::UnsupportedVersion {
            line: line_number,
            schema_version: schema_version.to_string(),
        });
    }

    serde_json::from_value(value)
        .map(Some)
        .map_err(|source| ProtocolError::InvalidJson {
            line: line_number,
            source,
        })
}

pub fn decode_events(input: &str) -> Result<Vec<AdapterEvent>, ProtocolError> {
    input
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| decode_event_line(line, idx + 1).transpose())
        .collect()
}

