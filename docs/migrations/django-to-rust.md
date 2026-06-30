# Django to Rust

Use this guide when Django/Python is the reference implementation and Rust is
the candidate implementation.

Start with HTTP boundary contracts. Move inward only when the Rust service owns
the same domain behavior and both sides can emit stable observations.

The runnable fixture is in [`examples/django-to-rust`](../../examples/django-to-rust).

## Migration Shape

```txt
Django view or API test
  -> Python/Django Rewrit observation
  -> stable case_id
  -> Rust cargo test observation
  -> Rewrit comparison report
```

The core rule is the same as every Rewrit migration: compare canonical
observations, not framework internals.

## Reference Side: Django

Use the Python SDK and Django helpers:

```python
from rewrit_pytest import rewrit_case
from rewrit_pytest.django import db_delta, observe_http_response


@rewrit_case("billing.invoice.create.success", suite_id="billing")
def test_creates_invoice(client):
    response = client.post(
        "/api/invoices",
        data={
            "customer_id": "cus_123",
            "amount": "199.90",
            "currency": "BRL",
        },
        content_type="application/json",
    )

    observe_http_response(
        response,
        effects=[
            db_delta(
                "invoices",
                inserted=[{
                    "customer_id": "cus_123",
                    "amount": "199.90",
                    "currency": "BRL",
                    "status": "open",
                }],
            )
        ],
    )
```

The pytest plugin emits `case_discovered` during collection and emits a fallback
pass/fail observation if a marked test does not emit one manually.

## Candidate Side: Rust

Use the Rust SDK from `cargo test`:

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

For HTTP-shaped comparisons, emit a canonical object with `status`, `headers`,
and `body`. See
[`examples/django-to-rust/candidate-rust/tests/billing.rs`](../../examples/django-to-rust/candidate-rust/tests/billing.rs).

## Manifest

```toml
[project]
name = "django-to-rust"
reference = "reference_django"
candidate = "candidate_rust"
contracts_dir = "contracts"
baselines_dir = ".rewrit/baselines"
reports_dir = ".rewrit/reports"

[runtimes.reference_django]
adapter = "command"
cwd = "reference-django"
command = ["python3", "run.py"]
timeout_ms = 30000

[runtimes.reference_django.protocol]
output = "file"

[runtimes.candidate_rust]
adapter = "rust:cargo_test"
cwd = "candidate-rust"
command = ["cargo", "test", "--", "--nocapture"]
timeout_ms = 30000

[runtimes.candidate_rust.protocol]
output = "file"
```

File output is recommended because pytest and `cargo test` both write runner
text to stdout.

## Workflow

1. Pick a Django API/view behavior with a stable input and output.
2. Add a Rewrit case ID to the Django test.
3. Emit HTTP response observations and required side effects.
4. Add a Rust test with the same case ID.
5. Run mirror mode:

   ```bash
   rewrit run --mode mirror
   ```

6. Add JSON contracts for behavior that should remain stable after the Django
   reference is removed.
7. Capture a baseline if the Django runtime becomes expensive to run in CI.

## Fixture

Run parity mode:

```bash
cargo run -p rewrit-cli -- run --manifest examples/django-to-rust/rewrit.toml
```

Run contract verification:

```bash
cargo run -p rewrit-cli -- verify \
  --manifest examples/django-to-rust/rewrit.toml \
  --contracts 'contracts/**/*.json'
```

Run the end-to-end regression:

```bash
cargo test -p rewrit-engine --test sdk_fixtures django_to_rust_fixture_runs_end_to_end -- --nocapture
```

The SDK-level regression for the pytest plugin and Django HTTP helper is:

```bash
cargo test -p rewrit-engine --test sdk_observations python_sdk_emits_pytest_and_django_observations -- --nocapture
```

## Common Pitfalls

- Do not compare Django model internals to Rust structs directly. Compare the
  boundary behavior.
- Keep generated request IDs, timestamps, and trace IDs behind scoped
  normalizers.
- Use canonical decimal strings for money.
- Prefer file protocol output for `cargo test`.
