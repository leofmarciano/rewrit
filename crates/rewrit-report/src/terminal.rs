use rewrit_model::{DivergenceKind, Report, Severity};
use std::collections::BTreeMap;

#[must_use]
pub fn render(report: &Report) -> String {
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    for divergence in &report.divergences {
        if matches!(divergence.severity, Severity::Blocking) {
            *by_kind.entry(kind_name(&divergence.kind).to_string()).or_default() += 1;
        }
    }

    let mut output = String::new();
    output.push_str("Rewrit parity report\n\n");
    output.push_str(&format!("Project: {}\n", report.project));
    output.push_str(&format!("Reference: {}\n", report.reference));
    output.push_str(&format!("Candidate: {}\n\n", report.candidate));
    output.push_str(&format!(
        "Cases discovered: {}\n",
        report.summary.cases_discovered
    ));
    output.push_str(&format!("Cases compared: {}\n", report.summary.cases_compared));
    output.push_str(&format!("Equivalent: {}\n", report.summary.equivalent));
    output.push_str(&format!("Allowed by waiver: {}\n", report.summary.waived));
    output.push_str(&format!(
        "Blocking divergences: {}\n",
        report.summary.blocking
    ));
    output.push_str(&format!("Parity: {:.2}%\n\n", report.summary.parity_ratio * 100.0));

    if !by_kind.is_empty() {
        output.push_str("Blocking:\n");
        for (kind, count) in by_kind {
            output.push_str(&format!("  {kind}: {count}\n"));
        }
        output.push('\n');
    }

    if !report.suites.is_empty() {
        output.push_str("Worst suites:\n");
        let mut suites = report.suites.clone();
        suites.sort_by(|left, right| {
            left.parity_ratio
                .partial_cmp(&right.parity_ratio)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for suite in suites.into_iter().take(5) {
            output.push_str(&format!(
                "  {}: {:.2}%\n",
                suite.suite_id,
                suite.parity_ratio * 100.0
            ));
        }
        output.push('\n');
    }

    output.push_str(&format!("Exit: {}\n", report.summary.exit_code));
    output
}

fn kind_name(kind: &DivergenceKind) -> &'static str {
    match kind {
        DivergenceKind::MissingCandidateCase => "missing_candidate_case",
        DivergenceKind::MissingReferenceCase => "missing_reference_case",
        DivergenceKind::OrphanCandidateCase => "orphan_candidate_case",
        DivergenceKind::OutputMismatch => "output_mismatch",
        DivergenceKind::TypeMismatch => "type_mismatch",
        DivergenceKind::SchemaMismatch => "schema_mismatch",
        DivergenceKind::ErrorMismatch => "error_mismatch",
        DivergenceKind::SideEffectMismatch => "side_effect_mismatch",
        DivergenceKind::StdoutMismatch => "stdout_mismatch",
        DivergenceKind::StderrMismatch => "stderr_mismatch",
        DivergenceKind::ExitCodeMismatch => "exit_code_mismatch",
        DivergenceKind::Timeout => "timeout",
        DivergenceKind::Flaky => "flaky",
        DivergenceKind::AdapterError => "adapter_error",
        DivergenceKind::InfraError => "infra_error",
        DivergenceKind::PolicyAllowed => "policy_allowed",
        DivergenceKind::WaiverExpired => "waiver_expired",
        _ => "unknown",
    }
}
