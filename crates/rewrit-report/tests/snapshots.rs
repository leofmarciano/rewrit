use rewrit_model::{
    CaseId, Divergence, DivergenceKind, Report, ReportSummary, Severity, SuiteSummary,
};
use std::collections::BTreeMap;

#[test]
fn terminal_report_snapshot() {
    insta::assert_snapshot!(rewrit_report::render("terminal", &sample_report()).expect("render"), @r###"
    Rewrit parity report

    Project: billing-migration
    Reference: legacy_laravel
    Candidate: encore_ts

    Cases discovered: 2
    Cases compared: 2
    Equivalent: 1
    Allowed by waiver: 0
    Blocking divergences: 1
    Parity: 50.00%

    Blocking:
      type_mismatch: 1

    Worst suites:
      billing: 50.00%

    Exit: 1
    "###);
}

#[test]
fn markdown_report_snapshot() {
    insta::assert_snapshot!(rewrit_report::render("markdown", &sample_report()).expect("render"), @r###"
    # Rewrit parity report

    - Project: `billing-migration`
    - Reference: `legacy_laravel`
    - Candidate: `encore_ts`
    - Parity: `50.00%`

    | Case | Kind | Severity | Path | Message |
    | --- | --- | --- | --- | --- |
    | `billing.invoice.create.success` | `TypeMismatch` | `Blocking` | `$.amount` | amount type differs |
    "###);
}

fn sample_report() -> Report {
    Report {
        schema_version: "rewrit.report.v1".to_string(),
        run_id: "run-1".to_string(),
        project: "billing-migration".to_string(),
        reference: "legacy_laravel".to_string(),
        candidate: "encore_ts".to_string(),
        summary: ReportSummary {
            cases_discovered: 2,
            cases_compared: 2,
            equivalent: 1,
            waived: 0,
            blocking: 1,
            warnings: 0,
            parity_ratio: 0.5,
            exit_code: 1,
        },
        suites: vec![SuiteSummary {
            suite_id: "billing".to_string(),
            cases_compared: 2,
            equivalent: 1,
            blocking: 1,
            parity_ratio: 0.5,
        }],
        divergences: vec![Divergence {
            kind: DivergenceKind::TypeMismatch,
            severity: Severity::Blocking,
            case_id: CaseId::new("billing.invoice.create.success"),
            suite: Some("billing".to_string()),
            path: Some("$.amount".to_string()),
            reference: Some(serde_json::json!("199.90")),
            candidate: Some(serde_json::json!(199.9)),
            message: "amount type differs".to_string(),
            machine_code: "type_mismatch".to_string(),
            source_location: None,
            target_location: None,
            policy: Some("strict".to_string()),
            normalizers_applied: Vec::new(),
            hint: Some("Return amount as a decimal string.".to_string()),
            minimal_reproduction: None,
        }],
        normalizers_applied: Vec::new(),
        policy_trace: Vec::new(),
        metadata: BTreeMap::new(),
    }
}
