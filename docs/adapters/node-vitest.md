# Node Vitest Adapter

The Vitest adapter exposes a reporter and a `createRewritTest(test)` helper.

```ts
import RewritVitestReporter from "@rewrit/node/vitest-reporter";

export default {
  test: {
    reporters: [new RewritVitestReporter()],
  },
};
```

Use the helper in tests:

```ts
import { test as baseTest } from "vitest";
import { createRewritTest, observe } from "@rewrit/node/vitest-reporter";

const test = createRewritTest(baseTest);

test.rewrit("billing.invoice.create.success", "creates an invoice", async () => {
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
