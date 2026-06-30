use rewrit_model::{DivergenceKind, Severity};

#[must_use]
pub fn default_severity(kind: &DivergenceKind) -> Severity {
    match kind {
        DivergenceKind::OrphanCandidateCase | DivergenceKind::PolicyAllowed => Severity::Warning,
        _ => Severity::Blocking,
    }
}

