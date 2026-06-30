use rewrit_model::Report;

#[must_use]
pub fn render(report: &Report) -> String {
    let mut output = String::new();
    output.push_str("# Rewrit parity report\n\n");
    output.push_str(&format!("- Project: `{}`\n", report.project));
    output.push_str(&format!("- Reference: `{}`\n", report.reference));
    output.push_str(&format!("- Candidate: `{}`\n", report.candidate));
    output.push_str(&format!(
        "- Parity: `{:.2}%`\n\n",
        report.summary.parity_ratio * 100.0
    ));
    output.push_str("| Case | Kind | Severity | Path | Message |\n");
    output.push_str("| --- | --- | --- | --- | --- |\n");
    for divergence in &report.divergences {
        output.push_str(&format!(
            "| `{}` | `{:?}` | `{:?}` | `{}` | {} |\n",
            divergence.case_id,
            divergence.kind,
            divergence.severity,
            divergence.path.as_deref().unwrap_or(""),
            divergence.message.replace('|', "\\|")
        ));
    }
    output
}
