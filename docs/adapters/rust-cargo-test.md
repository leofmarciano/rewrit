# Rust Cargo Test Adapter

The Rust SDK emits Rewrit protocol events directly from tests. Configure the
runtime as a command-compatible cargo-test adapter:

```toml
[runtimes.candidate]
adapter = "rust:cargo_test"
command = ["cargo", "test", "--", "--nocapture"]
timeout_ms = 30000
```

`--nocapture` is required because the SDK writes NDJSON events to stdout when
the engine is not using file transport.

```rust
#[test]
fn creates_invoice() -> Result<(), Box<dyn std::error::Error>> {
    rewrit::cargo_test_case("billing.invoice.create.success")?;

    let response = serde_json::json!({
        "amount": "199.90",
        "currency": "BRL",
        "status": "open"
    });

    rewrit::observe_json(&response)?;
    Ok(())
}
```

For side effects, use `rewrit::db_delta(...)` and `rewrit::add_effect(...)`, or
pass effects to `rewrit::observe_canonical(...)`.

A future proc-macro crate may provide `#[rewrit::case("case.id")]`, but the
explicit helper is the stable MVP surface.
