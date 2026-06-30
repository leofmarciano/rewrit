use crate::normalize::{NormalizeContext, NormalizeError, Normalizer};
use regex::Regex;
use rewrit_model::{CanonicalError, CanonicalValue, Observation};

#[derive(Debug, Clone)]
pub struct RegexNormalizer {
    name: &'static str,
    regex: Regex,
    replacement: String,
}

impl RegexNormalizer {
    pub fn new(
        name: &'static str,
        pattern: &str,
        replacement: impl Into<String>,
    ) -> Result<Self, regex::Error> {
        Ok(Self {
            name,
            regex: Regex::new(pattern)?,
            replacement: replacement.into(),
        })
    }

    pub fn uuid() -> Self {
        Self::new(
            "uuid",
            r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b",
            "<UUID>",
        )
        .expect("built-in UUID regex is valid")
    }
}

impl Normalizer for RegexNormalizer {
    fn name(&self) -> &'static str {
        self.name
    }

    fn normalize(
        &self,
        mut observation: Observation,
        _ctx: &NormalizeContext,
    ) -> Result<Observation, NormalizeError> {
        observation.stdout.text = self.replace(&observation.stdout.text);
        observation.stderr.text = self.replace(&observation.stderr.text);
        if let Some(value) = &mut observation.value {
            self.normalize_value(value);
        }
        if let Some(error) = &mut observation.error {
            self.normalize_error(error);
        }
        Ok(observation)
    }
}

impl RegexNormalizer {
    fn replace(&self, value: &str) -> String {
        self.regex
            .replace_all(value, self.replacement.as_str())
            .into_owned()
    }

    fn normalize_error(&self, error: &mut CanonicalError) {
        if let Some(message) = &mut error.message {
            *message = self.replace(message);
        }
        if let Some(message) = &mut error.normalized_message {
            *message = self.replace(message);
        }
    }

    fn normalize_value(&self, value: &mut CanonicalValue) {
        match value {
            CanonicalValue::String { value }
            | CanonicalValue::Integer { value }
            | CanonicalValue::Decimal { value }
            | CanonicalValue::Float { value }
            | CanonicalValue::DateTime { rfc3339: value } => {
                *value = self.replace(value);
            }
            CanonicalValue::Array { items } => {
                for item in items {
                    self.normalize_value(item);
                }
            }
            CanonicalValue::Object { fields } => {
                for field in fields.values_mut() {
                    self.normalize_value(field);
                }
            }
            CanonicalValue::Json { value } => normalize_json(value, &|s| self.replace(s)),
            CanonicalValue::Null
            | CanonicalValue::Absent
            | CanonicalValue::Bool { .. }
            | CanonicalValue::Bytes { .. } => {}
        }
    }
}

fn normalize_json(value: &mut serde_json::Value, replace: &dyn Fn(&str) -> String) {
    match value {
        serde_json::Value::String(text) => *text = replace(text),
        serde_json::Value::Array(items) => {
            for item in items {
                normalize_json(item, replace);
            }
        }
        serde_json::Value::Object(fields) => {
            for item in fields.values_mut() {
                normalize_json(item, replace);
            }
        }
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {}
    }
}
