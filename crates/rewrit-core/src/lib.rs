//! Pure Rewrit logic: normalization, comparison, policy, waivers and validation.
//!
//! This crate has no framework-specific knowledge and does not execute code.

#![forbid(unsafe_code)]

pub mod compare;
pub mod normalize;
pub mod policy;
pub mod validate;

pub use compare::{Comparator, CompareContext, Comparison, StrictComparator};
pub use normalize::{
    NormalizationPipeline, NormalizationResult, NormalizeContext, NormalizeError, Normalizer,
};
pub use policy::{Policy, PolicyEngine, Waiver, WaiverSet};
