use crate::normalize::{NormalizeContext, NormalizeError, Normalizer};
use rewrit_model::{CanonicalError, Observation, StackFrame};

#[derive(Debug, Clone)]
pub struct PathNormalizer {
    pub replacement: String,
}

impl Default for PathNormalizer {
    fn default() -> Self {
        Self {
            replacement: "<PROJECT_ROOT>".to_string(),
        }
    }
}

impl Normalizer for PathNormalizer {
    fn name(&self) -> &'static str {
        "path"
    }

    fn normalize(
        &self,
        mut observation: Observation,
        ctx: &NormalizeContext,
    ) -> Result<Observation, NormalizeError> {
        let Some(root) = &ctx.project_root else {
            return Ok(observation);
        };

        observation.stdout.text = observation.stdout.text.replace(root, &self.replacement);
        observation.stderr.text = observation.stderr.text.replace(root, &self.replacement);

        if let Some(error) = &mut observation.error {
            normalize_error(error, root, &self.replacement);
        }

        for artifact in &mut observation.artifacts {
            artifact.path = artifact.path.replace(root, &self.replacement);
        }

        Ok(observation)
    }
}

fn normalize_error(error: &mut CanonicalError, root: &str, replacement: &str) {
    for StackFrame { file, .. } in &mut error.frames {
        if let Some(file) = file {
            *file = file.replace(root, replacement);
        }
    }
}
