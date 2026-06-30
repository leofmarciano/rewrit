# Node Vitest Adapter

Use the Vitest integration when a Node or TypeScript runtime already has Vitest
tests that can emit Rewrit observations.

The SDK provides:

- `createRewritTest(test)` for stable case discovery,
- `observe(...)` for JSON-shaped observations,
- `observeCanonical(...)` for explicit canonical values,
- `RewritVitestReporter` for a lightweight doctor event.

## Manifest

Run Vitest through the command adapter:

```toml
[runtimes.candidate]
adapter = "command"
cwd = "../candidate"
command = ["npx", "vitest", "run", "--reporter=default"]
timeout_ms = 30000

[runtimes.candidate.protocol]
output = "file"
```

File output is recommended so Vitest's normal reporter output does not mix with
protocol events on stdout.

## Vitest Configuration

```ts
import RewritVitestReporter from "@rewrit/node/vitest-reporter";

export default {
  test: {
    reporters: ["default", new RewritVitestReporter()],
  },
};
```

## Test Usage

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

`test.rewrit(...)` emits a `case_discovered` event and makes the case ID current
for later `observe(...)` calls.

## Notes

- Use stable domain case IDs, not Vitest test names.
- Use `observeCanonical(...)` when JSON is not precise enough for the value
  type you need.
- Attach side effects with helpers from framework-specific modules, such as
  `@rewrit/node/encore`, or by emitting canonical observations directly.
