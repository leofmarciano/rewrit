use crate::escape_xml;
use rewrit_model::{Report, Severity};

#[must_use]
pub fn render(report: &Report) -> String {
    let failures = report
        .divergences
        .iter()
        .filter(|divergence| matches!(divergence.severity, Severity::Blocking))
        .count();
    let tests = report.summary.cases_compared.max(report.summary.cases_discovered);
    let mut output = String::new();
    output.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    output.push('\n');
    output.push_str(&format!(
        r#"<testsuite name="rewrit" tests="{tests}" failures="{failures}">"#
    ));
    output.push('\n');
    for divergence in &report.divergences {
        output.push_str(&format!(
            r#"  <testcase classname="rewrit.{}" name="{}">"#,
            escape_xml(divergence.suite.as_deref().unwrap_or("default")),
            escape_xml(divergence.case_id.as_str())
        ));
        output.push('\n');
        if matches!(divergence.severity, Severity::Blocking) {
            output.push_str(&format!(
                r#"    <failure type="{:?}" message="{}">{}</failure>"#,
                divergence.kind,
                escape_xml(&divergence.message),
                escape_xml(&serde_json::to_string(divergence).unwrap_or_default())
            ));
            output.push('\n');
        }
        output.push_str("  </testcase>\n");
    }
    output.push_str("</testsuite>\n");
    output
}

