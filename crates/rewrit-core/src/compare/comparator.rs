use crate::compare::diff::{divergence, value_divergences};
use crate::compare::effects::effects_equivalent;
use crate::compare::error::errors_equivalent;
use crate::policy::Policy;
use rewrit_model::{CaseId, Divergence, DivergenceKind, Observation, Severity, SourceLocation};

#[derive(Debug, Clone)]
pub struct CompareContext {
    pub policy: Policy,
    pub suite: Option<String>,
    pub source_location: Option<SourceLocation>,
    pub target_location: Option<SourceLocation>,
    pub normalizers_applied: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Comparison {
    pub case_id: CaseId,
    pub equivalent: bool,
    pub divergences: Vec<Divergence>,
}

pub trait Comparator: Send + Sync {
    fn name(&self) -> &'static str;

    fn compare(
        &self,
        reference: &Observation,
        candidate: &Observation,
        ctx: &CompareContext,
    ) -> Comparison;
}

#[derive(Debug, Default)]
pub struct StrictComparator;

impl Comparator for StrictComparator {
    fn name(&self) -> &'static str {
        "strict"
    }

    fn compare(
        &self,
        reference: &Observation,
        candidate: &Observation,
        ctx: &CompareContext,
    ) -> Comparison {
        let mut divergences = Vec::new();

        if reference.status != candidate.status {
            divergences.push(divergence(
                DivergenceKind::OutputMismatch,
                reference.case_id.clone(),
                "$.status",
                "Runtime statuses differ.",
                Some(&reference.status),
                Some(&candidate.status),
                ctx,
            ));
        }

        if ctx.policy.compare_exit_code && reference.exit_code != candidate.exit_code {
            divergences.push(divergence(
                DivergenceKind::ExitCodeMismatch,
                reference.case_id.clone(),
                "$.exit_code",
                "Runtime exit codes differ.",
                Some(&reference.exit_code),
                Some(&candidate.exit_code),
                ctx,
            ));
        }

        match (&reference.value, &candidate.value) {
            (Some(left), Some(right)) => divergences.extend(value_divergences(
                &reference.case_id,
                left,
                right,
                "$.value",
                ctx,
            )),
            (Some(left), None) => divergences.push(divergence(
                DivergenceKind::OutputMismatch,
                reference.case_id.clone(),
                "$.value",
                "Reference produced a value but candidate did not.",
                Some(left),
                Option::<&serde_json::Value>::None,
                ctx,
            )),
            (None, Some(right)) => divergences.push(divergence(
                DivergenceKind::OutputMismatch,
                reference.case_id.clone(),
                "$.value",
                "Candidate produced a value but reference did not.",
                Option::<&serde_json::Value>::None,
                Some(right),
                ctx,
            )),
            _ => {}
        }

        if !errors_equivalent(
            reference.error.as_ref(),
            candidate.error.as_ref(),
            &ctx.policy,
        ) {
            divergences.push(divergence(
                DivergenceKind::ErrorMismatch,
                reference.case_id.clone(),
                "$.error",
                "Canonical errors differ.",
                reference.error.as_ref(),
                candidate.error.as_ref(),
                ctx,
            ));
        }

        if !effects_equivalent(&reference.effects, &candidate.effects, &ctx.policy) {
            divergences.push(divergence(
                DivergenceKind::SideEffectMismatch,
                reference.case_id.clone(),
                "$.effects",
                "Observed side effects differ.",
                Some(&reference.effects),
                Some(&candidate.effects),
                ctx,
            ));
        }

        if ctx.policy.compare_stdout && reference.stdout != candidate.stdout {
            divergences.push(divergence(
                DivergenceKind::StdoutMismatch,
                reference.case_id.clone(),
                "$.stdout",
                "Captured stdout differs.",
                Some(&reference.stdout),
                Some(&candidate.stdout),
                ctx,
            ));
        }

        if ctx.policy.compare_stderr && reference.stderr != candidate.stderr {
            divergences.push(divergence(
                DivergenceKind::StderrMismatch,
                reference.case_id.clone(),
                "$.stderr",
                "Captured stderr differs.",
                Some(&reference.stderr),
                Some(&candidate.stderr),
                ctx,
            ));
        }

        for divergence in &mut divergences {
            if matches!(
                divergence.kind,
                DivergenceKind::Timeout | DivergenceKind::AdapterError | DivergenceKind::InfraError
            ) {
                divergence.severity = Severity::Blocking;
            }
        }

        Comparison {
            case_id: reference.case_id.clone(),
            equivalent: divergences.is_empty(),
            divergences,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Comparator, CompareContext, StrictComparator};
    use crate::policy::Policy;
    use rewrit_model::{
        CanonicalValue, CapturedText, CaseId, CaseStatus, DivergenceKind, Observation, RuntimeId,
    };
    use std::collections::BTreeMap;

    #[test]
    fn detects_json_type_mismatch_at_path() {
        let comparison = StrictComparator.compare(
            &observation(json_value(serde_json::json!({"amount": "199.90"}))),
            &observation(json_value(serde_json::json!({"amount": 199.9}))),
            &ctx(Policy::default()),
        );

        assert_eq!(comparison.divergences.len(), 1);
        assert_eq!(comparison.divergences[0].kind, DivergenceKind::TypeMismatch);
        assert_eq!(
            comparison.divergences[0].path.as_deref(),
            Some("$.value.amount")
        );
        assert!(comparison.divergences[0]
            .hint
            .as_deref()
            .is_some_and(|hint| hint.contains("$.value.amount")));
    }

    #[test]
    fn ignores_configured_json_path() {
        let comparison = StrictComparator.compare(
            &observation(json_value(serde_json::json!({"trace_id": "a", "ok": true}))),
            &observation(json_value(serde_json::json!({"trace_id": "b", "ok": true}))),
            &ctx(Policy {
                ignore_paths: vec!["$.trace_id".to_string()],
                ..Policy::default()
            }),
        );

        assert!(comparison.equivalent);
        assert!(comparison.divergences.is_empty());
    }

    #[test]
    fn treats_configured_json_array_path_as_unordered() {
        let comparison = StrictComparator.compare(
            &observation(json_value(serde_json::json!({
                "items": [{"id": 2}, {"id": 1}]
            }))),
            &observation(json_value(serde_json::json!({
                "items": [{"id": 1}, {"id": 2}]
            }))),
            &ctx(Policy {
                unordered_paths: vec!["$.items".to_string()],
                ..Policy::default()
            }),
        );

        assert!(comparison.equivalent);
        assert!(comparison.divergences.is_empty());
    }

    #[test]
    fn ignores_configured_http_header_noise() {
        let reference = CanonicalValue::Object {
            fields: BTreeMap::from([
                (
                    "headers".to_string(),
                    CanonicalValue::Object {
                        fields: BTreeMap::from([(
                            "date".to_string(),
                            CanonicalValue::String {
                                value: "Mon, 01 Jan 2024 00:00:00 GMT".to_string(),
                            },
                        )]),
                    },
                ),
                (
                    "body".to_string(),
                    json_value(serde_json::json!({"ok": true})),
                ),
            ]),
        };
        let candidate = CanonicalValue::Object {
            fields: BTreeMap::from([
                (
                    "headers".to_string(),
                    CanonicalValue::Object {
                        fields: BTreeMap::from([(
                            "date".to_string(),
                            CanonicalValue::String {
                                value: "Tue, 02 Jan 2024 00:00:00 GMT".to_string(),
                            },
                        )]),
                    },
                ),
                (
                    "body".to_string(),
                    json_value(serde_json::json!({"ok": true})),
                ),
            ]),
        };

        let comparison = StrictComparator.compare(
            &observation(reference),
            &observation(candidate),
            &ctx(Policy::default()),
        );

        assert!(comparison.equivalent);
        assert!(comparison.divergences.is_empty());
    }

    fn ctx(policy: Policy) -> CompareContext {
        CompareContext {
            policy,
            suite: None,
            source_location: None,
            target_location: None,
            normalizers_applied: Vec::new(),
        }
    }

    fn observation(value: CanonicalValue) -> Observation {
        Observation {
            case_id: CaseId::new("case.one"),
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

    fn json_value(value: serde_json::Value) -> CanonicalValue {
        CanonicalValue::Json { value }
    }
}
