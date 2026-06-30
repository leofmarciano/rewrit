# Python Pytest Adapter

Use the pytest plugin when a Python runtime can emit Rewrit observations from
pytest tests.

The package exposes a `pytest11` entry point and the helpers:

- `@rewrit_case(case_id, suite_id=None, title=None)`,
- `emit_observation(value=None, case_id=None, status="passed", effects=None)`,
- `emit_canonical_observation(...)`,
- `add_effect(...)`.

## Manifest

```toml
[runtimes.reference]
adapter = "command"
cwd = "../reference"
command = ["python3", "-m", "pytest", "-q"]
timeout_ms = 30000

[runtimes.reference.protocol]
output = "file"
```

File output is recommended because pytest writes runner output to stdout.

## Decorator Usage

```python
from rewrit_pytest import emit_observation, rewrit_case


@rewrit_case("billing.invoice.create.success", suite_id="billing")
def test_creates_invoice(client):
    response = client.post("/api/invoices", json={
        "customer_id": "cus_123",
        "amount": "199.90",
        "currency": "BRL",
    })

    emit_observation(response.json())
```

## Marker Usage

```python
import pytest
from rewrit_pytest import emit_observation


@pytest.mark.rewrit_case("billing.invoice.create.success", suite_id="billing")
def test_creates_invoice(client):
    emit_observation({"status": "open"})
```

During collection, the plugin emits `case_discovered` events for marked tests.
During execution, it sets the current Rewrit case. If a marked test does not
emit an observation, the plugin emits a fallback pass/fail observation.

## Notes

- Use [Django adapter](django.md) helpers for Django response and DB delta
  capture.
- Use `emit_canonical_observation(...)` when type precision matters.
- Keep test assertions in pytest; Rewrit observations are for cross-runtime
  comparison.
