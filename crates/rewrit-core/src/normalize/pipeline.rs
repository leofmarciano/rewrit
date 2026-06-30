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

    fn paths(&self) -> &[String] {
        &[]
    }

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
                if normalizer.paths().is_empty() {
                    applied.push(AppliedNormalizer {
                        case_id: observation.case_id.clone(),
                        name: normalizer.name().to_string(),
                        path: None,
                    });
                } else {
                    applied.extend(normalizer.paths().iter().cloned().map(|path| {
                        AppliedNormalizer {
                            case_id: observation.case_id.clone(),
                            name: normalizer.name().to_string(),
                            path: Some(path),
                        }
                    }));
                }
            }
        }

        Ok(NormalizationResult {
            observation,
            applied,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::regex::RegexNormalizer;
    use rewrit_model::{CanonicalValue, CapturedText, CaseId, CaseStatus, RuntimeId};
    use std::collections::BTreeMap;

    #[test]
    fn applied_normalizer_records_configured_paths() {
        let pipeline = NormalizationPipeline::new(vec![Box::new(
            RegexNormalizer::new("test", "token-[0-9]+", "<TOKEN>")
                .expect("regex")
                .with_paths(vec!["$.token".to_string()]),
        )]);
        let observation = Observation {
            case_id: CaseId::new("auth.case"),
            runtime_id: RuntimeId::new("reference"),
            status: CaseStatus::Passed,
            value: Some(CanonicalValue::Json {
                value: serde_json::json!({"token": "token-123"}),
            }),
            error: None,
            stdout: CapturedText::default(),
            stderr: CapturedText::default(),
            exit_code: Some(0),
            duration_ms: 1,
            effects: Vec::new(),
            artifacts: Vec::new(),
            metadata: BTreeMap::new(),
        };

        let result = pipeline
            .normalize(observation, &NormalizeContext::default())
            .expect("normalize");

        assert_eq!(result.applied.len(), 1);
        assert_eq!(result.applied[0].name, "test");
        assert_eq!(result.applied[0].path.as_deref(), Some("$.token"));
    }
}
