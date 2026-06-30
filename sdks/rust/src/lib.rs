//! Rust SDK helpers for emitting Rewrit observations.

#![forbid(unsafe_code)]

use rewrit_model::{CapturedText, CaseId, CaseStatus, Observation, RuntimeId};
use rewrit_protocol::{encode_event_line, AdapterEvent};
use std::collections::BTreeMap;

pub fn observation(case_id: impl Into<String>, runtime_id: impl Into<String>) -> Observation {
    Observation {
        case_id: CaseId::new(case_id),
        runtime_id: RuntimeId::new(runtime_id),
        status: CaseStatus::Passed,
        value: None,
        error: None,
        stdout: CapturedText::default(),
        stderr: CapturedText::default(),
        exit_code: Some(0),
        duration_ms: 0,
        effects: Vec::new(),
        artifacts: Vec::new(),
        metadata: BTreeMap::new(),
    }
}

pub fn emit_observation(observation: Observation) -> Result<(), serde_json::Error> {
    print!(
        "{}",
        encode_event_line(&AdapterEvent::observation(observation))?
    );
    Ok(())
}

pub mod macros {
    pub const FUTURE_CASE_MACRO: &str = "#[rewrit::case]";
}
