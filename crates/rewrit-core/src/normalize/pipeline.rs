use rewrit_model::{AppliedNormalizer, Observation};
use thiserror::Error;

#[derive(Debug, Clone, Default)]
pub struct NormalizeContext {
    pub project_root: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NormalizationResult {
    pub observation: Observation,
    pub applied: Vec<AppliedNormalizer>,
}

#[derive(Debug, Error)]
pub enum NormalizeError {
    #[error("normalizer {name} failed: {message}")]
    Failed { name: String, message: String },
}

pub trait Normalizer: Send + Sync {
    fn name(&self) -> &'static str;

    fn normalize(
        &self,
        observation: Observation,
        ctx: &NormalizeContext,
    ) -> Result<Observation, NormalizeError>;
}

#[derive(Default)]
pub struct NormalizationPipeline {
    normalizers: Vec<Box<dyn Normalizer>>,
}

impl NormalizationPipeline {
    #[must_use]
    pub fn new(normalizers: Vec<Box<dyn Normalizer>>) -> Self {
        Self { normalizers }
    }

    pub fn push(&mut self, normalizer: Box<dyn Normalizer>) {
        self.normalizers.push(normalizer);
    }

    pub fn normalize(
        &self,
        mut observation: Observation,
        ctx: &NormalizeContext,
    ) -> Result<NormalizationResult, NormalizeError> {
        let mut applied = Vec::new();
        for normalizer in &self.normalizers {
            let before = observation.clone();
            observation = normalizer.normalize(observation, ctx)?;
            if observation != before {
                applied.push(AppliedNormalizer {
                    case_id: observation.case_id.clone(),
                    name: normalizer.name().to_string(),
                    path: None,
                });
            }
        }

        Ok(NormalizationResult {
            observation,
            applied,
        })
    }
}
