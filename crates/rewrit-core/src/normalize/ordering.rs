use crate::normalize::{NormalizeContext, NormalizeError, Normalizer};
use rewrit_model::Observation;

#[derive(Debug, Clone, Default)]
pub struct OrderingNormalizer;

impl Normalizer for OrderingNormalizer {
    fn name(&self) -> &'static str {
        "ordering"
    }

    fn normalize(
        &self,
        observation: Observation,
        _ctx: &NormalizeContext,
    ) -> Result<Observation, NormalizeError> {
        // Canonical object values are already stored in BTreeMap. Array ordering
        // remains significant unless a future policy marks a path unordered.
        Ok(observation)
    }
}
