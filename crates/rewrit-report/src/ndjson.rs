use rewrit_model::Report;

pub fn render(report: &Report) -> Result<String, serde_json::Error> {
    let mut output = String::new();
    output.push_str(&serde_json::to_string(&report.summary)?);
    output.push('\n');
    for divergence in &report.divergences {
        output.push_str(&serde_json::to_string(divergence)?);
        output.push('\n');
    }
    Ok(output)
}

