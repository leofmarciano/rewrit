# Node Jest Adapter

Use the Jest integration when a Node runtime already has Jest tests that can
emit Rewrit observations.

The SDK provides:

- `rewrit(caseId, name, fn)` to register a Jest test with a stable case ID,
- `observe(...)` for JSON-shaped observations,
- `observeCanonical(...)` for explicit canonical values,
- `RewritJestReporter` for a lightweight doctor event.

## Manifest

```toml
[runtimes.candidate]
adapter = "command"
cwd = "../candidate"
command = ["npx", "jest", "--runInBand"]
timeout_ms = 30000

[runtimes.candidate.protocol]
output = "file"
```

File output avoids mixing Jest's human output with Rewrit protocol events.

## Jest Configuration

```js
export default {
  reporters: ["default", "@rewrit/node/jest-reporter"],
};
```

## Test Usage

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

The helper uses the global Jest `test(...)` function. If your project wraps the
test API, pass it as the fourth argument:

```ts
rewrit("case.id", "title", async () => {
  observe({ ok: true });
}, customTest);
```

## Notes

- Prefer one Rewrit case per domain behavior.
- Keep assertions in Jest; Rewrit observes behavior for cross-runtime
  comparison.
- Use `REWRIT_EVENTS_PATH` file output for real test suites.
