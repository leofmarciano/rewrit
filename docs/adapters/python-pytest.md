# Python Pytest Adapter

The pytest adapter marks cases with `@rewrit_case("case.id")` and emits
observations through the Python SDK.

The plugin is exposed through the `pytest11` entry point. During collection it
emits `case_discovered` events for marked tests. During test execution it sets
the current Rewrit case and emits a basic pass/fail observation if the test did
not call `emit_observation(...)` manually.

```python
from rewrit_pytest import emit_observation, rewrit_case


@rewrit_case("billing.invoice.create.success")
def test_creates_invoice(client):
    response = client.post("/api/invoices", json={
        "customer_id": "cus_123",
        "amount": "199.90",
        "currency": "BRL",
    })

    emit_observation(response.json())
```

The same case can be marked with pytest's marker syntax:

```python
import pytest


@pytest.mark.rewrit_case("billing.invoice.create.success", suite_id="billing")
def test_creates_invoice(client):
    ...
```

The SDK writes NDJSON to stdout by default and appends to `REWRIT_EVENTS_PATH`
when the command adapter uses file transport.
