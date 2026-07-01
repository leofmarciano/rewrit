use rewrit_model::{Report, Severity};
use serde_json::json;

pub fn render(report: &Report) -> Result<String, serde_json::Error> {
    let results: Vec<_> = report
        .divergences
        .iter()
        .filter(|divergence| matches!(divergence.severity, Severity::Blocking))
        .map(|divergence| {
            let location = divergence
                .target_location
                .as_ref()
                .or(divergence.source_location.as_ref());
            json!({
                "ruleId": format!("{:?}", divergence.kind),
                "level": "error",
                "message": { "text": divergence.message },
                "locations": location.map(|location| vec![json!({
                    "physicalLocation": {
                        "artifactLocation": { "uri": location.path },
                        "region": { "startLine": location.line.unwrap_or(1) }
                    }
                })]).unwrap_or_default()
            })
        })
        .collect();

    serde_json::to_string_pretty(&json!({
        "version": "2.1.0",
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "rewrit",
                    "informationUri": "https://github.com/leofmarciano/rewrit",
                    "rules": []
                }
            },
            "results": results,
            "properties": {
                "project": report.project,
                "reference": report.reference,
                "candidate": report.candidate
            }
        }]
    }))
}
