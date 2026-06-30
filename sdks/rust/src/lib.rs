//! Rust SDK helpers for emitting Rewrit observations.

#![forbid(unsafe_code)]

use rewrit_model::{
    CanonicalValue, CapturedText, Case, CaseId, CaseStatus, Effect, Observation, RuntimeId, SuiteId,
};
use rewrit_protocol::{encode_event_line, AdapterEvent, EVENT_SCHEMA_VERSION};
use serde::Serialize;
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Default)]
struct State {
    current_case_id: Option<CaseId>,
    current_suite_id: Option<SuiteId>,
    last_observation: Option<Observation>,
}

#[derive(Debug)]
pub enum EmitError {
    MissingCaseId,
    StatePoisoned,
    Json(serde_json::Error),
    Io(std::io::Error),
}

impl fmt::Display for EmitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCaseId => formatter.write_str("Rewrit case id is missing"),
            Self::StatePoisoned => formatter.write_str("Rewrit SDK state lock was poisoned"),
            Self::Json(error) => write!(formatter, "failed to encode Rewrit event: {error}"),
            Self::Io(error) => write!(formatter, "failed to write Rewrit event: {error}"),
        }
    }
}

impl Error for EmitError {}

impl From<serde_json::Error> for EmitError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<std::io::Error> for EmitError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

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

pub fn runtime_id() -> RuntimeId {
    RuntimeId::new(env::var("REWRIT_RUNTIME_ID").unwrap_or_else(|_| "candidate".to_string()))
}

pub fn cargo_test_case(case_id: impl Into<String>) -> Result<(), EmitError> {
    case_discovered(case_id)
}

pub fn case_discovered(case_id: impl Into<String>) -> Result<(), EmitError> {
    let case_id = case_id.into();
    let suite_id = suite_from_case_id(&case_id);
    case_discovered_with(case_id.clone(), suite_id, case_id)
}

pub fn case_discovered_with(
    case_id: impl Into<String>,
    suite_id: impl Into<String>,
    title: impl Into<String>,
) -> Result<(), EmitError> {
    let case_id = CaseId::new(case_id);
    let suite_id = SuiteId::new(suite_id);
    let case = Case {
        id: case_id.clone(),
        suite_id: suite_id.clone(),
        title: title.into(),
        source_location: None,
        tags: Vec::new(),
        contract_ref: None,
        required: true,
    };
    with_state(|state| {
        state.current_case_id = Some(case_id);
        state.current_suite_id = Some(suite_id);
    })?;
    emit_event(&AdapterEvent::CaseDiscovered {
        schema_version: EVENT_SCHEMA_VERSION.to_string(),
        runtime_id: runtime_id(),
        case,
    })
}

pub fn observe_json<T: Serialize>(value: &T) -> Result<(), EmitError> {
    let value = CanonicalValue::Json {
        value: serde_json::to_value(value)?,
    };
    observe_canonical(Some(value), CaseStatus::Passed, Vec::new())
}

pub fn observe_canonical(
    value: Option<CanonicalValue>,
    status: CaseStatus,
    effects: Vec<Effect>,
) -> Result<(), EmitError> {
    let case_id = current_case_id()?;
    observe_canonical_for(case_id, value, status, effects)
}

pub fn observe_canonical_for(
    case_id: CaseId,
    value: Option<CanonicalValue>,
    status: CaseStatus,
    effects: Vec<Effect>,
) -> Result<(), EmitError> {
    let mut observation = Observation {
        case_id,
        runtime_id: runtime_id(),
        status,
        value,
        error: None,
        stdout: CapturedText::default(),
        stderr: CapturedText::default(),
        exit_code: Some(0),
        duration_ms: 0,
        effects,
        artifacts: Vec::new(),
        metadata: BTreeMap::new(),
    };
    if let Some(suite_id) = current_suite_id()? {
        observation
            .metadata
            .insert("suite_id".to_string(), suite_id.to_string());
    }
    emit_observation(observation)
}

pub fn emit_observation(observation: Observation) -> Result<(), EmitError> {
    with_state(|state| {
        state.last_observation = Some(observation.clone());
    })?;
    emit_event(&AdapterEvent::observation(observation))
}

pub fn add_effect(effect: Effect) -> Result<(), EmitError> {
    let case_id = current_case_id()?;
    add_effect_for(case_id, effect)
}

pub fn add_effect_for(case_id: CaseId, effect: Effect) -> Result<(), EmitError> {
    let updated = with_state(|state| {
        if let Some(last_observation) = &mut state.last_observation {
            if last_observation.case_id == case_id {
                last_observation.effects.push(effect.clone());
                return Some(last_observation.clone());
            }
        }
        None
    })?;

    if let Some(observation) = updated {
        emit_event(&AdapterEvent::observation(observation))
    } else {
        observe_canonical_for(case_id, None, CaseStatus::Passed, vec![effect])
    }
}

pub fn db_delta(
    table: impl Into<String>,
    inserted: Vec<BTreeMap<String, CanonicalValue>>,
    updated: Vec<BTreeMap<String, CanonicalValue>>,
    deleted: Vec<BTreeMap<String, CanonicalValue>>,
    connection: impl Into<String>,
) -> Effect {
    Effect::DbDelta(rewrit_model::DbDelta {
        connection: connection.into(),
        table: table.into(),
        inserted,
        updated,
        deleted,
    })
}

pub fn canonical_json<T: Serialize>(value: &T) -> Result<CanonicalValue, EmitError> {
    Ok(CanonicalValue::Json {
        value: serde_json::to_value(value)?,
    })
}

fn emit_event(event: &AdapterEvent) -> Result<(), EmitError> {
    let encoded = encode_event_line(event)?;
    if let Ok(events_path) = env::var("REWRIT_EVENTS_PATH") {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(events_path)?;
        file.write_all(encoded.as_bytes())?;
        return Ok(());
    }

    std::io::stdout().write_all(encoded.as_bytes())?;
    Ok(())
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(State::default()))
}

fn with_state<T>(operation: impl FnOnce(&mut State) -> T) -> Result<T, EmitError> {
    let mut guard = state().lock().map_err(|_| EmitError::StatePoisoned)?;
    Ok(operation(&mut guard))
}

fn current_case_id() -> Result<CaseId, EmitError> {
    with_state(|state| state.current_case_id.clone())?.ok_or(EmitError::MissingCaseId)
}

fn current_suite_id() -> Result<Option<SuiteId>, EmitError> {
    with_state(|state| state.current_suite_id.clone())
}

fn suite_from_case_id(case_id: &str) -> String {
    case_id
        .split_once('.')
        .map(|(suite, _)| suite.to_string())
        .unwrap_or_else(|| "default".to_string())
}

pub mod macros {
    pub const FUTURE_CASE_MACRO: &str = "#[rewrit::case]";
}
