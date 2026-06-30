//! Rewrit orchestration engine.
//!
//! The engine loads manifests, runs adapters, stores baselines, invokes the pure
//! core and writes reports. It does not contain framework-specific adapter code.

#![forbid(unsafe_code)]

pub mod discovery;
pub mod engine;
pub mod events;
pub mod planner;
pub mod runner;
pub mod scheduler;
pub mod store;

pub use discovery::manifest::ProjectConfig;
pub use engine::{Engine, EngineError, EngineOptions, ExplainResult, Manifest, RunMode};
pub use runner::process::{ProcessOutput, ProcessRunner, RuntimeProcess};
