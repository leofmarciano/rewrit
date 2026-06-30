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
    baseline_dir(store, runtime_id).join("current.jsonl")
}

pub fn baseline_dir(store: &RewritStore, runtime_id: &RuntimeId) -> PathBuf {
    store.baselines_dir.join(runtime_id.as_str())
}

pub fn write_current(
    store: &RewritStore,
    runtime_id: &RuntimeId,
    observations: &[Observation],
) -> Result<PathBuf, BaselineError> {
    let path = current_path(store, runtime_id);
    let dir = baseline_dir(store, runtime_id);
    std::fs::create_dir_all(&dir).map_err(|source| BaselineError::Io {
        path: dir.display().to_string(),
        source,
    })?;

    let mut output = String::new();
    for observation in observations {
        output.push_str(&encode_event_line(&AdapterEvent::observation(
            observation.clone(),
        ))?);
    }
    let timestamped_path = unique_timestamped_path(&dir);
    std::fs::write(&timestamped_path, &output).map_err(|source| BaselineError::Io {
        path: timestamped_path.display().to_string(),
        source,
    })?;
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

fn unique_timestamped_path(dir: &std::path::Path) -> PathBuf {
    let timestamp = baseline_timestamp(time::OffsetDateTime::now_utc());
    let mut path = dir.join(format!("{timestamp}.jsonl"));
    let mut attempt = 1usize;
    while path.exists() {
        path = dir.join(format!("{timestamp}-{attempt}.jsonl"));
        attempt += 1;
    }
    path
}

fn baseline_timestamp(now: time::OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}-{:02}-{:02}Z",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rewrit_model::{CapturedText, CaseId, CaseStatus};
    use std::collections::BTreeMap;

    #[test]
    fn write_current_persists_current_and_timestamped_baseline() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = RewritStore::new(temp.path(), None, None);
        store.ensure().expect("store");
        let runtime_id = RuntimeId::new("reference");
        let observations = vec![Observation {
            case_id: CaseId::new("billing.invoice.create.success"),
            runtime_id: runtime_id.clone(),
            status: CaseStatus::Passed,
            value: None,
            error: None,
            stdout: CapturedText::default(),
            stderr: CapturedText::default(),
            exit_code: Some(0),
            duration_ms: 3,
            effects: Vec::new(),
            artifacts: Vec::new(),
            metadata: BTreeMap::new(),
        }];

        let current = write_current(&store, &runtime_id, &observations).expect("write");
        let dir = baseline_dir(&store, &runtime_id);
        let timestamped = std::fs::read_dir(&dir)
            .expect("read baseline dir")
            .map(|entry| entry.expect("entry").path())
            .filter(|path| path.file_name().and_then(|name| name.to_str()) != Some("current.jsonl"))
            .collect::<Vec<_>>();

        assert!(current.ends_with("current.jsonl"));
        assert_eq!(timestamped.len(), 1);
        assert_eq!(
            std::fs::read_to_string(&current).expect("current contents"),
            std::fs::read_to_string(&timestamped[0]).expect("snapshot contents")
        );
        assert_eq!(read_current(&store, &runtime_id).expect("read").len(), 1);
    }
}
