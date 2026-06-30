use rewrit_model::Report;

pub fn render(report: &Report) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}
