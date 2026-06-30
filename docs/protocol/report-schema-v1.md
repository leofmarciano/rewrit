# Report Schema v1

The report schema describes machine-readable Rewrit output. It is intended for
CI systems, dashboards, pull request annotations, and code agents.

Generate the current JSON Schema:

```bash
cargo run -p rewrit-cli -- schema export --kind report
```

Generate it into a directory:

```bash
cargo run -p rewrit-cli -- schema export --kind report --out-dir dist/schemas
```

## Top-Level Shape

Reports include:

- `schema_version`,
- `run_id`,
- `project`,
- `reference`,
- `candidate`,
- `summary`,
- `suites`,
- `divergences`,
- `normalizers_applied`,
- `policy_trace`,
- `metadata`.

## Summary

`summary` gives the CI-level answer:

```json
{
  "cases_discovered": 2,
  "cases_compared": 2,
  "equivalent": 1,
  "waived": 0,
  "blocking": 1,
  "warnings": 0,
  "parity_ratio": 0.5,
  "exit_code": 1
}
```

`exit_code = 0` means no blocking divergence remains.

## Divergences

Each divergence carries enough context to locate and fix the mismatch:

```json
{
  "kind": "type_mismatch",
  "severity": "blocking",
  "case_id": "billing.invoice.create.success",
  "suite": "billing",
  "path": "$.body.amount",
  "reference": { "type": "string", "value": "199.90" },
  "candidate": { "type": "number", "value": 199.9 },
  "message": "Candidate returned number, but reference returned string.",
  "machine_code": "type_mismatch",
  "source_location": null,
  "target_location": null,
  "policy": "http_api_strict",
  "normalizers_applied": [],
  "hint": "Keep money as a decimal string.",
  "minimal_reproduction": {
    "command": "rewrit",
    "args": ["explain", "billing.invoice.create.success"],
    "cwd": "."
  }
}
```

Common divergence kinds include:

- `missing_candidate_case`,
- `missing_reference_case`,
- `orphan_candidate_case`,
- `output_mismatch`,
- `type_mismatch`,
- `schema_mismatch`,
- `error_mismatch`,
- `side_effect_mismatch`,
- `stdout_mismatch`,
- `stderr_mismatch`,
- `exit_code_mismatch`,
- `timeout`,
- `flaky`,
- `adapter_error`,
- `infra_error`,
- `policy_allowed`,
- `waiver_expired`.

## Report Formats

The report crate supports terminal, JSON, NDJSON, JUnit XML, SARIF, HTML, and
Markdown report targets. Use JSON when another tool needs the full structure.
Use JUnit or SARIF when integrating with existing CI annotation surfaces.
