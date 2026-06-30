# Observations

An observation is the canonical runtime output for one case.

Adapters emit observations so the Rust engine can compare behavior without
knowing the framework that produced it.

## Minimal Observation

```json
{
  "schema_version": "rewrit.event.v1",
  "kind": "observation",
  "case_id": "billing.invoice.create.success",
  "runtime_id": "reference",
  "status": "passed",
  "value": {
    "kind": "json",
    "value": {
      "id": "inv_123",
      "amount": "199.90",
      "currency": "BRL",
      "status": "open"
    }
  },
  "error": null,
  "stdout": { "text": "", "truncated": false },
  "stderr": { "text": "", "truncated": false },
  "exit_code": 0,
  "duration_ms": 12,
  "effects": [],
  "artifacts": [],
  "metadata": {}
}
```

## Fields

- `case_id`: stable behavior ID.
- `runtime_id`: manifest runtime ID, normally the reference or candidate.
- `status`: `passed`, `failed`, `skipped`, `timed_out`, `adapter_error`, or
  `infra_error`.
- `value`: canonical value produced by the runtime.
- `error`: canonical error when the case failed in an expected or meaningful
  way.
- `stdout` and `stderr`: captured text with truncation flags.
- `exit_code`: process exit code when available.
- `duration_ms`: runtime duration for the case.
- `effects`: side effects that are part of the behavior contract.
- `artifacts`: files or generated outputs attached to the observation.
- `metadata`: small string key/value context for reports.

## Canonical Values

Rewrit does not reduce every value to raw JSON. The canonical value model
distinguishes:

- `null`,
- `absent`,
- `bool`,
- `integer`,
- `decimal`,
- `float`,
- `string`,
- `bytes`,
- `array`,
- `object`,
- `date_time`,
- `json`.

That distinction catches migration bugs that raw string or JSON comparison often
misses. For example, `"199.90"` and `199.9` are different by default because
money represented as a float is not equivalent to a decimal string.

## HTTP-Shaped Values

SDK helpers for Laravel, Django, Encore, and HTTP-style tests emit values with
this shape:

```json
{
  "kind": "object",
  "fields": {
    "status": { "kind": "integer", "value": "201" },
    "headers": {
      "kind": "object",
      "fields": {
        "content-type": { "kind": "string", "value": "application/json" }
      }
    },
    "body": {
      "kind": "json",
      "value": {
        "status": "open"
      }
    }
  }
}
```

This keeps HTTP parity consistent even when one side is a framework test client
and the other side is a Fetch-compatible response.

## Best Practices

- Emit `case_discovered` before the observation when your runner can discover
  cases.
- Emit one final observation per case. SDKs may re-emit an updated observation
  when side effects are added after the value.
- Use `REWRIT_EVENTS_PATH` file output for noisy test runners such as
  `cargo test` or any runner that writes human output to stdout.
- Keep large blobs in artifacts and put a digest or summary in `value`.
- Put suite information in discovery events or `metadata.suite_id`.
