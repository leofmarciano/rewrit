# Node Encore Adapter

The Encore helper focuses on HTTP and service-boundary observations for
Encore.ts migrations.

Encore.ts projects should not rely on `.env` loading for Rewrit. The command
adapter injects `REWRIT_RUNTIME_ID` and `REWRIT_EVENTS_PATH` into the process
environment, while Encore application secrets/config remain owned by Encore's
runtime configuration.

```ts
import { encoreCase, observeHttpResponse, observeServiceResult } from "@rewrit/node/encore";

test("creates invoice", async () => {
  encoreCase("billing.invoice.create.success");

  const result = await invoiceService.create({
    customer_id: "cus_123",
    amount: "199.90",
    currency: "BRL",
  });

  observeServiceResult(result);
});
```

For HTTP boundary tests, pass a Fetch-compatible response:

```ts
import { encoreCase, observeHttpResponse } from "@rewrit/node/encore";

test("creates invoice over HTTP", async () => {
  encoreCase("billing.invoice.create.success");

  const response = await fetch("http://127.0.0.1:4000/api/invoices", {
    method: "POST",
    body: JSON.stringify({
      customer_id: "cus_123",
      amount: "199.90",
      currency: "BRL",
    }),
  });

  await observeHttpResponse(response);
});
```

Side effects can be attached with `observeDbDelta(...)` or by passing `dbDelta`
effects into the observation helpers.
