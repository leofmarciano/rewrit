use crate::compare::CompareContext;
use rewrit_model::{CanonicalValue, CaseId, Divergence, DivergenceKind, Severity};
use serde::Serialize;

pub fn divergence<T, U>(
    kind: DivergenceKind,
    case_id: CaseId,
    path: impl Into<String>,
    message: impl Into<String>,
    reference: Option<&T>,
    candidate: Option<&U>,
    ctx: &CompareContext,
) -> Divergence
where
    T: Serialize + ?Sized,
    U: Serialize + ?Sized,
{
    Divergence {
        machine_code: format!("{kind:?}").to_ascii_lowercase(),
        kind,
        severity: Severity::Blocking,
        case_id,
        suite: ctx.suite.clone(),
        path: Some(path.into()),
        reference: reference.and_then(|value| serde_json::to_value(value).ok()),
        candidate: candidate.and_then(|value| serde_json::to_value(value).ok()),
        message: message.into(),
        source_location: ctx.source_location.clone(),
        target_location: ctx.target_location.clone(),
        policy: Some(ctx.policy.name.clone()),
        normalizers_applied: ctx.normalizers_applied.clone(),
        hint: None,
    }
}

#[must_use]
pub fn value_type_name(value: &CanonicalValue) -> &'static str {
    value.kind_name()
}

