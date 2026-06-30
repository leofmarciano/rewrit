use crate::compare::diff::{divergence, value_type_name};
use crate::compare::error::errors_equivalent;
use crate::policy::Policy;
use rewrit_model::{
    CaseId, Divergence, DivergenceKind, Observation, Severity, SourceLocation,
};

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
            (Some(left), Some(right)) if !ctx.policy.values_equivalent(left, right) => {
                let kind = if value_type_name(left) != value_type_name(right) {
                    DivergenceKind::TypeMismatch
                } else {
                    DivergenceKind::OutputMismatch
                };
                divergences.push(divergence(
                    kind,
                    reference.case_id.clone(),
                    "$.value",
                    "Canonical output values differ.",
                    Some(left),
                    Some(right),
                    ctx,
                ));
            }
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

        if !errors_equivalent(reference.error.as_ref(), candidate.error.as_ref(), &ctx.policy) {
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

        if reference.effects != candidate.effects {
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

