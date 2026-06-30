use crate::normalize::{NormalizeContext, NormalizeError, Normalizer};
use regex::Regex;
use rewrit_model::{CanonicalError, CanonicalValue, Observation};

#[derive(Debug, Clone)]
pub struct RegexNormalizer {
    name: &'static str,
    regex: Regex,
    replacement: String,
    paths: Vec<String>,
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
            paths: Vec::new(),
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

    #[must_use]
    pub fn with_paths(mut self, paths: Vec<String>) -> Self {
        self.paths = paths;
        self
    }
}

impl Normalizer for RegexNormalizer {
    fn name(&self) -> &'static str {
        self.name
    }

    fn paths(&self) -> &[String] {
        &self.paths
    }

    fn normalize(
        &self,
        mut observation: Observation,
        _ctx: &NormalizeContext,
    ) -> Result<Observation, NormalizeError> {
        if self.paths.is_empty() {
            observation.stdout.text = self.replace(&observation.stdout.text);
            observation.stderr.text = self.replace(&observation.stderr.text);
            if let Some(value) = &mut observation.value {
                self.normalize_value(value);
            }
            if let Some(error) = &mut observation.error {
                self.normalize_error(error);
            }
        } else if let Some(value) = &mut observation.value {
            self.normalize_value_at_paths(value, "$");
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

    fn normalize_value_at_paths(&self, value: &mut CanonicalValue, path: &str) {
        if self.path_matches(path) {
            self.normalize_value(value);
            return;
        }

        match value {
            CanonicalValue::Array { items } => {
                for (index, item) in items.iter_mut().enumerate() {
                    self.normalize_value_at_paths(item, &format!("{path}[{index}]"));
                }
            }
            CanonicalValue::Object { fields } => {
                for (key, value) in fields {
                    self.normalize_value_at_paths(value, &format!("{path}.{}", escape_path(key)));
                }
            }
            CanonicalValue::Json { value } => self.normalize_json_at_paths(value, path),
            CanonicalValue::Null
            | CanonicalValue::Absent
            | CanonicalValue::Bool { .. }
            | CanonicalValue::Integer { .. }
            | CanonicalValue::Decimal { .. }
            | CanonicalValue::Float { .. }
            | CanonicalValue::String { .. }
            | CanonicalValue::Bytes { .. }
            | CanonicalValue::DateTime { .. } => {}
        }
    }

    fn normalize_json_at_paths(&self, value: &mut serde_json::Value, path: &str) {
        if self.path_matches(path) {
            normalize_json(value, &|s| self.replace(s));
            return;
        }

        match value {
            serde_json::Value::Array(items) => {
                for (index, item) in items.iter_mut().enumerate() {
                    self.normalize_json_at_paths(item, &format!("{path}[{index}]"));
                }
            }
            serde_json::Value::Object(fields) => {
                for (key, value) in fields {
                    self.normalize_json_at_paths(value, &format!("{path}.{}", escape_path(key)));
                }
            }
            serde_json::Value::Null
            | serde_json::Value::Bool(_)
            | serde_json::Value::Number(_)
            | serde_json::Value::String(_) => {}
        }
    }

    fn path_matches(&self, path: &str) -> bool {
        candidate_policy_paths(path).iter().any(|candidate| {
            self.paths.iter().any(|configured| {
                candidate == configured || wildcard_path_matches(candidate, configured)
            })
        })
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

fn candidate_policy_paths(path: &str) -> Vec<String> {
    let mut paths = vec![path.to_string()];
    if let Some(suffix) = path.strip_prefix("$.body") {
        if suffix.is_empty() {
            paths.push("$".to_string());
        } else if let Some(suffix) = suffix.strip_prefix('.') {
            paths.push(format!("$.{suffix}"));
        } else if suffix.starts_with('[') {
            paths.push(format!("${suffix}"));
        }
    }
    if let Some(suffix) = path.strip_prefix("$.value.body") {
        if suffix.is_empty() {
            paths.push("$".to_string());
        } else if let Some(suffix) = suffix.strip_prefix('.') {
            paths.push(format!("$.{suffix}"));
        } else if suffix.starts_with('[') {
            paths.push(format!("${suffix}"));
        }
    }
    if let Some(suffix) = path.strip_prefix("$.value") {
        if suffix.is_empty() {
            paths.push("$".to_string());
        } else if let Some(suffix) = suffix.strip_prefix('.') {
            paths.push(format!("$.{suffix}"));
        } else if suffix.starts_with('[') {
            paths.push(format!("${suffix}"));
        }
    }
    paths
}

fn wildcard_path_matches(path: &str, configured: &str) -> bool {
    if !configured.contains("[*]") {
        return false;
    }
    let prefix = configured.split("[*]").next().unwrap_or_default();
    let suffix = configured.split("[*]").nth(1).unwrap_or_default();
    path.starts_with(prefix) && path.ends_with(suffix)
}

fn escape_path(segment: &str) -> String {
    if segment
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        segment.to_string()
    } else {
        format!("[{}]", serde_json::to_string(segment).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rewrit_model::{CapturedText, CaseId, CaseStatus, RuntimeId};
    use std::collections::BTreeMap;

    #[test]
    fn scoped_regex_normalizes_only_configured_json_paths() {
        let normalizer = RegexNormalizer::new("test", r"\bid-[0-9]+\b", "<ID>")
            .expect("regex")
            .with_paths(vec!["$.items[*].id".to_string()]);
        let mut observation = observation_with_value(CanonicalValue::Json {
            value: serde_json::json!({
                "items": [
                    {"id": "id-123", "label": "id-123"},
                    {"id": "id-456", "label": "id-456"}
                ],
                "id": "id-789"
            }),
        });

        observation = normalizer
            .normalize(observation, &NormalizeContext::default())
            .expect("normalize");

        assert_eq!(
            observation.value,
            Some(CanonicalValue::Json {
                value: serde_json::json!({
                    "items": [
                        {"id": "<ID>", "label": "id-123"},
                        {"id": "<ID>", "label": "id-456"}
                    ],
                    "id": "id-789"
                })
            })
        );
    }

    fn observation_with_value(value: CanonicalValue) -> Observation {
        Observation {
            case_id: CaseId::new("case"),
            runtime_id: RuntimeId::new("runtime"),
            status: CaseStatus::Passed,
            value: Some(value),
            error: None,
            stdout: CapturedText::default(),
            stderr: CapturedText::default(),
            exit_code: Some(0),
            duration_ms: 1,
            effects: Vec::new(),
            artifacts: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }
}
