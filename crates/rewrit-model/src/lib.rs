//! Canonical Rewrit data model.
//!
//! This crate is intentionally passive: it defines identifiers, contracts,
//! observations, divergences and reports. It does not execute runtimes, parse
//! manifests, normalize values or compare behavior.

#![forbid(unsafe_code)]

pub mod case;
pub mod contract;
pub mod divergence;
pub mod effect;
pub mod error;
pub mod ids;
pub mod observation;
pub mod report;
pub mod value;

pub use case::{Case, ContractRef, SourceLocation};
pub use contract::{Contract, ContractExpectation, ContractInput};
pub use divergence::{Divergence, DivergenceKind, MinimalReproduction, Severity};
pub use effect::*;
pub use error::{CanonicalError, ErrorKind, StackFrame};
pub use ids::{AdapterId, CaseId, RuntimeId, SuiteId};
pub use observation::{Artifact, CapturedText, CaseStatus, Observation};
pub use report::{AppliedNormalizer, PolicyDecision, Report, ReportSummary, SuiteSummary};
pub use value::CanonicalValue;
