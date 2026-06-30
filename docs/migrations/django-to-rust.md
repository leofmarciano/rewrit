# Django To Rust

Use boundary contracts first:

```txt
Django view/API
  -> Rewrit HTTP observation
  -> contract case_id
  -> Rust service/API test
  -> Rewrit Rust observation
```

The core rule is the same as every Rewrit migration: the engine compares
canonical observations, not framework internals. Django and Rust code only need
to agree on stable `case_id` values and the observable contract.

## Reference Side: Pytest/Django

Use the Python SDK and pytest plugin for Django reference behavior:

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
pass/fail observation when a marked test does not emit one manually. The Django
helper emits canonical HTTP values with `status`, lowercase `headers`, JSON body
when available, and attached effects.

## Candidate Side: Rust

Use the Rust SDK from `cargo test`:

```rust
#[rewrit::case("billing.invoice.create.success")]
#[test]
fn creates_invoice() -> Result<(), Box<dyn std::error::Error>> {
    let body = serde_json::json!({
        "id": "inv_123",
        "amount": "199.90",
        "currency": "BRL",
        "status": "open"
    });

    rewrit::observe_json(&body)?;
    Ok(())
}
```

For HTTP-shaped comparisons, emit a canonical object with `status`, `headers`
and `body`, as shown in `examples/django-to-rust/candidate-rust/tests/billing.rs`.

## Manifest

Use `command` for the Django reference and `rust:cargo_test` for the candidate:

```toml
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

File output is recommended because both pytest and `cargo test` can print runner
text to stdout.

## Executable Fixture

The guide is backed by `examples/django-to-rust`.

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

Run the repository regression that executes this fixture end to end:

```bash
cargo test -p rewrit-engine --test sdk_fixtures django_to_rust_fixture_runs_end_to_end -- --nocapture
```

The SDK-level regression for the pytest plugin and Django HTTP helper is:

```bash
cargo test -p rewrit-engine --test sdk_observations python_sdk_emits_pytest_and_django_observations -- --nocapture
```
