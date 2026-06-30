# Django Adapter

Django support builds on the Python pytest plugin. Start at HTTP boundaries
before instrumenting domain internals.

The Django helpers live in `rewrit_pytest.django` and intentionally avoid a hard
import of Django internals. They operate on common Django test client response
shapes.

## Manifest

```toml
[runtimes.reference_django]
adapter = "command"
cwd = "../reference-django"
command = ["python3", "-m", "pytest", "-q"]
timeout_ms = 30000

[runtimes.reference_django.protocol]
output = "file"
```

## HTTP Response Example

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

`observe_http_response(...)` emits:

- `status` as a canonical integer,
- lowercase response headers,
- JSON body when parsing succeeds,
- string body when parsing fails,
- attached effects when provided.

`observe_db_delta(...)` can be used when you want to add a DB effect after the
observation:

```python
from rewrit_pytest.django import observe_db_delta

observe_db_delta("invoices", inserted=[{"status": "open"}])
```

## Notes

- Prefer API/view tests as the first migration boundary.
- Capture database deltas only for behavior that downstream systems rely on.
- Keep Django-specific assertions in pytest; Rewrit compares the canonical
  observation against the candidate runtime.
