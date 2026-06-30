# Observation Schema v1

The observation schema describes the canonical value emitted by an adapter for a
single case. Observation events use this schema inside
`kind = "observation"` adapter events.

Generate the current JSON Schema:

```bash
cargo run -p rewrit-cli -- schema export --kind observation
```

Generate it into a directory:

```bash
cargo run -p rewrit-cli -- schema export --kind observation --out-dir dist/schemas
```

## Required Fields

| Field | Description |
| --- | --- |
| `case_id` | Stable case ID shared by reference, candidate, contracts, reports, and waivers. |
| `runtime_id` | Runtime ID from `rewrit.toml`. |
| `status` | Case status: `passed`, `failed`, `skipped`, `timed_out`, `adapter_error`, or `infra_error`. |
| `value` | Optional canonical value. |
| `error` | Optional canonical error. |
| `stdout` | Captured stdout text and truncation flag. |
| `stderr` | Captured stderr text and truncation flag. |
| `exit_code` | Process exit code when available. |
| `duration_ms` | Case duration in milliseconds. |
| `effects` | Side effects emitted during the case. |
| `artifacts` | File artifacts attached to the case. |
| `metadata` | Small string key/value metadata. |

## Canonical Values

`value` is one of the canonical value kinds:

```txt
null
absent
bool
integer
decimal
float
string
bytes
array
object
date_time
json
```

Use canonical kinds when type precision matters. Use `kind = "json"` when the
native JSON shape is already the contract you want to compare.

## Example

```json
{
  "case_id": "billing.invoice.create.success",
  "runtime_id": "candidate",
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
  "duration_ms": 9,
  "effects": [],
  "artifacts": [],
  "metadata": { "suite_id": "billing" }
}
```

For the full event wrapper, see [Adapter protocol v1](adapter-protocol-v1.md).
