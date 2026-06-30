# Node Jest Adapter

The Jest adapter exposes a reporter and a `rewrit(case_id, name, fn)` helper.

Configure the reporter:

```js
export default {
  reporters: ["default", "@rewrit/node/jest-reporter"],
};
```

Use the helper in tests:

```ts
import { observe, rewrit } from "@rewrit/node/jest-reporter";

rewrit("billing.invoice.create.success", "creates an invoice", async () => {
  const response = {
    id: "inv_123",
    amount: "199.90",
    currency: "BRL",
    status: "open",
  };

  observe(response);
});
```

The SDK writes NDJSON to stdout by default and appends to `REWRIT_EVENTS_PATH`
when the command adapter uses file transport.
