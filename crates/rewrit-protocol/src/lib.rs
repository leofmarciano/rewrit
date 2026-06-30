//! Versioned NDJSON protocol shared by Rewrit adapters and the engine.

#![forbid(unsafe_code)]

pub mod adapter;
pub mod events;
pub mod ndjson;
pub mod version;

pub use adapter::{AdapterCommand, AdapterRequest};
pub use events::{AdapterEvent, DoctorReport};
pub use ndjson::{decode_event_line, decode_events, encode_event_line, ProtocolError};
pub use version::{
    ADAPTER_REQUEST_SCHEMA_VERSION, EVENT_SCHEMA_VERSION, OBSERVATION_SCHEMA_VERSION,
    REPORT_SCHEMA_VERSION,
};

