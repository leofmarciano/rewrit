use crate::store::filesystem::RewritStore;
use rewrit_model::{Observation, RuntimeId};
use rewrit_protocol::{decode_events, encode_event_line, AdapterEvent};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BaselineError {
    #[error("failed to access baseline file {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to encode baseline: {0}")]
    Encode(#[from] serde_json::Error),
    #[error("failed to parse baseline protocol: {0}")]
    Protocol(#[from] rewrit_protocol::ProtocolError),
}

pub fn current_path(store: &RewritStore, runtime_id: &RuntimeId) -> PathBuf {
    store.baselines_dir.join(runtime_id.as_str()).join("current.jsonl")
}

pub fn write_current(
    store: &RewritStore,
    runtime_id: &RuntimeId,
    observations: &[Observation],
) -> Result<PathBuf, BaselineError> {
    let path = current_path(store, runtime_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| BaselineError::Io {
            path: parent.display().to_string(),
            source,
        })?;
    }

    let mut output = String::new();
    for observation in observations {
        output.push_str(&encode_event_line(&AdapterEvent::observation(
            observation.clone(),
        ))?);
    }
    std::fs::write(&path, output).map_err(|source| BaselineError::Io {
        path: path.display().to_string(),
        source,
    })?;
    Ok(path)
}

pub fn read_current(
    store: &RewritStore,
    runtime_id: &RuntimeId,
) -> Result<Vec<Observation>, BaselineError> {
    let path = current_path(store, runtime_id);
    let input = std::fs::read_to_string(&path).map_err(|source| BaselineError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let observations = decode_events(&input)?
        .into_iter()
        .filter_map(|event| match event {
            AdapterEvent::Observation { observation, .. } => Some(observation),
            _ => None,
        })
        .collect();
    Ok(observations)
}

