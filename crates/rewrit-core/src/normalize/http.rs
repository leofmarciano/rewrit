use crate::normalize::{NormalizeContext, NormalizeError, Normalizer};
use rewrit_model::{Effect, HttpCall, Observation};

#[derive(Debug, Clone, Default)]
pub struct HttpHeaderNormalizer;

impl Normalizer for HttpHeaderNormalizer {
    fn name(&self) -> &'static str {
        "http_headers"
    }

    fn normalize(
        &self,
        mut observation: Observation,
        _ctx: &NormalizeContext,
    ) -> Result<Observation, NormalizeError> {
        for effect in &mut observation.effects {
            if let Effect::HttpCall(call) = effect {
                normalize_call(call);
            }
        }
        Ok(observation)
    }
}

fn normalize_call(call: &mut HttpCall) {
    call.request_headers = call
        .request_headers
        .iter()
        .map(|(key, value)| (key.to_ascii_lowercase(), value.clone()))
        .collect();
    call.response_headers = call
        .response_headers
        .iter()
        .map(|(key, value)| (key.to_ascii_lowercase(), value.clone()))
        .collect();
}
