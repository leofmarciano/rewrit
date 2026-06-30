# Rust Cargo Test Adapter

Use the Rust SDK when a Rust candidate or reference emits observations from
`cargo test`.

The SDK provides:

- `#[rewrit::case("case.id")]`,
- `rewrit::cargo_test_case(...)`,
- `rewrit::observe_json(...)`,
- `rewrit::observe_canonical(...)`,
- `rewrit::db_delta(...)`,
- `rewrit::add_effect(...)`.

## Manifest

```toml
[runtimes.candidate_rust]
adapter = "rust:cargo_test"
cwd = "../candidate-rust"
command = ["cargo", "test", "--", "--nocapture"]
timeout_ms = 30000

[runtimes.candidate_rust.protocol]
output = "file"
```

File output is recommended because `cargo test` writes harness text to stdout.
`--nocapture` is useful when tests emit events during execution.

## Attribute Usage

```rust
#[rewrit::case("billing.invoice.create.success")]
#[test]
fn creates_invoice() -> Result<(), Box<dyn std::error::Error>> {
    let response = serde_json::json!({
        "id": "inv_123",
        "amount": "199.90",
        "currency": "BRL",
        "status": "open"
    });

    rewrit::observe_json(&response)?;
    Ok(())
}
```

The attribute emits `case_discovered` before the test body runs.

## Explicit Helper Usage

```rust
#[test]
fn creates_invoice() -> Result<(), Box<dyn std::error::Error>> {
    rewrit::cargo_test_case("billing.invoice.create.success")?;
    rewrit::observe_json(&serde_json::json!({ "status": "open" }))?;
    Ok(())
}
```

## Side Effects

```rust
use std::collections::BTreeMap;

let mut row = BTreeMap::new();
row.insert(
    "amount".to_string(),
    rewrit::CanonicalValue::Decimal {
        value: "199.90".to_string(),
    },
);

rewrit::add_effect(rewrit::db_delta(
    "invoices",
    vec![row],
    vec![],
    vec![],
    "default",
))?;
```

## Notes

- Use `observe_json(...)` for ordinary JSON responses.
- Use `observe_canonical(...)` when type precision matters.
- Keep Rust assertions in the test; Rewrit observations are for comparison with
  the other runtime.
