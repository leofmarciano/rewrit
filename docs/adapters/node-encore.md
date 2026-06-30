# Node Encore Adapter

Use the Encore helpers for Encore.ts candidates or Node service boundaries that
need HTTP-shaped or service-result observations.

The SDK exports from `@rewrit/node/encore`:

- `encoreCase(caseId, suiteId?, title?)`,
- `observeServiceResult(value, caseId?, effects?)`,
- `observeHttpResponse(response, caseId?, effects?)`,
- `observeDbDelta(table, rows, connection?, caseId?)`,
- `dbDelta(table, rows, connection?)`.

## Manifest

```toml
[runtimes.encore_ts]
adapter = "command"
cwd = "../candidate"
command = ["npm", "run", "test:rewrit"]
timeout_ms = 30000

[runtimes.encore_ts.env]
NODE_ENV = "test"

[runtimes.encore_ts.protocol]
output = "file"
```

Rewrit injects protocol variables such as `REWRIT_RUNTIME_ID` and
`REWRIT_EVENTS_PATH`. Encore application configuration should remain owned by
Encore, not by Rewrit.

## Service Boundary Example

```ts
import {
  encoreCase,
  observeDbDelta,
  observeServiceResult,
} from "@rewrit/node/encore";

test("creates invoice", async () => {
  encoreCase("billing.invoice.create.success", "billing", "creates invoice");

  const result = await invoiceService.create({
    customer_id: "cus_123",
    amount: "199.90",
    currency: "BRL",
  });

  observeServiceResult(result);
  observeDbDelta("billing_invoices", {
    inserted: [{
      customer_ref: "cus_123",
      total_amount: "199.90",
      currency_code: "BRL",
      state: "open",
    }],
  });
});
```

If `observeDbDelta(...)` is called after `observeServiceResult(...)`, the SDK
emits an updated observation for the same case with the effect attached.

## HTTP Boundary Example

```ts
import { encoreCase, observeHttpResponse } from "@rewrit/node/encore";

test("creates invoice over HTTP", async () => {
  encoreCase("billing.invoice.create.success", "billing");

  const response = await fetch("http://127.0.0.1:4000/api/invoices", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      customer_id: "cus_123",
      amount: "199.90",
      currency: "BRL",
    }),
  });

  await observeHttpResponse(response);
});
```

`observeHttpResponse(...)` emits a canonical object with `status`, lowercase
`headers`, and a `body` parsed as JSON when possible.

## Notes

- Start with HTTP or service-boundary cases before instrumenting internals.
- Use database maps in `rewrit.toml` when the candidate schema differs from the
  reference schema.
- Keep volatile Encore/runtime fields behind path-scoped normalizers.
