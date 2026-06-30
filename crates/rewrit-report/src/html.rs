use crate::escape_xml;
use rewrit_model::Report;

#[must_use]
pub fn render(report: &Report) -> String {
    let mut output = String::new();
    output.push_str(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Rewrit report</title>",
    );
    output.push_str("<style>body{font-family:system-ui,sans-serif;margin:2rem;line-height:1.45}table{border-collapse:collapse;width:100%}td,th{border:1px solid #ddd;padding:.5rem;text-align:left}code{background:#f5f5f5;padding:.1rem .25rem}</style>");
    output.push_str("</head><body>");
    output.push_str(&format!("<h1>{}</h1>", escape_xml(&report.project)));
    output.push_str(&format!(
        "<p><strong>Reference:</strong> {}<br><strong>Candidate:</strong> {}<br><strong>Parity:</strong> {:.2}%</p>",
        escape_xml(&report.reference),
        escape_xml(&report.candidate),
        report.summary.parity_ratio * 100.0
    ));
    output.push_str("<table><thead><tr><th>Case</th><th>Kind</th><th>Severity</th><th>Path</th><th>Message</th></tr></thead><tbody>");
    for divergence in &report.divergences {
        output.push_str(&format!(
            "<tr><td><code>{}</code></td><td>{:?}</td><td>{:?}</td><td>{}</td><td>{}</td></tr>",
            escape_xml(divergence.case_id.as_str()),
            divergence.kind,
            divergence.severity,
            escape_xml(divergence.path.as_deref().unwrap_or("")),
            escape_xml(&divergence.message)
        ));
    }
    output.push_str("</tbody></table></body></html>");
    output
}
