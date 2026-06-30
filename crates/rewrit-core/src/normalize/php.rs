use crate::normalize::{NormalizeContext, NormalizeError, Normalizer};
use rewrit_model::Observation;

#[derive(Debug, Clone, Default)]
pub struct PhpArrayNormalizer;

impl Normalizer for PhpArrayNormalizer {
    fn name(&self) -> &'static str {
        "php_array"
    }

    fn normalize(
        &self,
        observation: Observation,
        _ctx: &NormalizeContext,
    ) -> Result<Observation, NormalizeError> {
        Ok(observation)
    }
}

