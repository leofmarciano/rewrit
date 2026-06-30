# Rust Cargo Test Adapter

The Rust SDK emits Rewrit protocol events directly from tests. Configure the
runtime as a command-compatible cargo-test adapter:

```toml
[runtimes.candidate]
adapter = "rust:cargo_test"
command = ["cargo", "test", "--", "--nocapture"]
timeout_ms = 30000

[runtimes.candidate.protocol]
output = "file"
```

File output is recommended because `cargo test` writes harness text to stdout.
`--nocapture` is still useful when a test emits events before the file transport
environment is configured.

```rust
#[rewrit::case("billing.invoice.create.success")]
#[test]
fn creates_invoice() -> Result<(), Box<dyn std::error::Error>> {
    let response = serde_json::json!({
        "amount": "199.90",
        "currency": "BRL",
        "status": "open"
    });

    rewrit::observe_json(&response)?;
    Ok(())
}
```

The explicit helper remains available for projects that do not want an attribute
macro:

```rust
#[test]
fn creates_invoice() -> Result<(), Box<dyn std::error::Error>> {
    rewrit::cargo_test_case("billing.invoice.create.success")?;
    rewrit::observe_json(&serde_json::json!({ "status": "open" }))?;
    Ok(())
}
```

For side effects, use `rewrit::db_delta(...)` and `rewrit::add_effect(...)`, or
pass effects to `rewrit::observe_canonical(...)`.
