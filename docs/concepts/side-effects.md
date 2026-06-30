# Side Effects

Parity is not only the returned value.

Many rewrite bugs appear in the effects around a response: a row is inserted
with the wrong field, a queue message disappears, an event changes shape, or an
email is sent twice. Rewrit models those effects as first-class observation
data.

## Supported Effect Kinds

The canonical model includes:

- `db_delta`: inserted, updated, and deleted rows.
- `file_delta`: created, updated, and deleted files.
- `http_call`: outbound HTTP calls.
- `queue_message`: queue or topic payloads.
- `event`: emitted domain events.
- `email`: recipient, subject, and body summary.
- `cache_operation`: cache reads/writes/deletes.
- `log`: structured logs that are part of the contract.

## Database Delta Example

```json
{
  "kind": "db_delta",
  "connection": "default",
  "table": "invoices",
  "inserted": [
    {
      "customer_id": { "kind": "string", "value": "cus_123" },
      "amount": { "kind": "string", "value": "199.90" },
      "currency": { "kind": "string", "value": "BRL" },
      "status": { "kind": "string", "value": "open" }
    }
  ],
  "updated": [],
  "deleted": []
}
```

SDK helpers expose this as `observeDbDelta(...)`, `observe_db_delta(...)`, or
`rewrit::db_delta(...)` depending on the language.

## Mapping Different Schemas

The candidate database does not need to copy the reference schema. Declare field
maps when the same domain effect is represented differently:

```toml
[effects.db.maps.invoices]
target_table = "billing_invoices"

[effects.db.maps.invoices.fields]
id = "invoice_id"
customer_id = "customer_ref"
amount = "total_amount"
currency = "currency_code"
status = "state"
```

This keeps the comparison focused on behavior instead of table naming.

## When to Track Side Effects

Track side effects when they are part of the observable contract:

- invoices must create ledger rows,
- refunds must emit events,
- registration must send email,
- imports must write files,
- API calls must enqueue jobs.

Do not track incidental implementation details unless a downstream system relies
on them. Over-modeling side effects makes migrations noisy and harder to
complete.
