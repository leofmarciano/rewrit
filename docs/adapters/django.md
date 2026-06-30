# Django Adapter

Django support should start at HTTP boundary contracts before moving into
domain internals.

The Python SDK exposes Django helpers from `rewrit_pytest.django`. They do not
import Django directly; they use the response shape exposed by Django's test
client and regular `HttpResponse` objects.

```python
from rewrit_pytest import rewrit_case
from rewrit_pytest.django import observe_db_delta, observe_http_response


@rewrit_case("billing.invoice.create.success")
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

    observe_http_response(response)
    observe_db_delta(
        "invoices",
        inserted=[{
            "customer_id": "cus_123",
            "amount": "199.90",
            "currency": "BRL",
            "status": "open",
        }],
    )
```

`observe_http_response()` emits a canonical HTTP value with `status`, lowercase
headers, and a JSON body when the response can be parsed as JSON. Otherwise it
falls back to a string body.

`observe_db_delta()` appends a `db_delta` side effect to the current case. It can
also be passed to `observe_http_response(..., effects=[db_delta(...)])` when a
single observation event is preferred.
