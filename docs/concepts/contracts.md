# Contracts

Contracts describe inputs, expected outputs, errors, HTTP details, side effects
and the policy used to decide whether differences are meaningful.

HTTP contracts use `kind = "http_case"` and are executed by the built-in HTTP
adapter. Non-HTTP contracts use the same file format with kinds such as
`command_case`, `job_case`, or `function_case`. In that mode, Rewrit passes the
selected contract IDs to command-compatible adapters through `AdapterRequest.cases`
and the `REWRIT_CASES` environment variable. The adapter is responsible for
running the matching command, job, or function and emitting observations with the
same `case_id`.

```json
{
  "schema_version": "rewrit.contract.v1",
  "id": "math.double.success",
  "kind": "function_case",
  "input": {
    "json": { "value": 2 }
  },
  "expect": {
    "json": { "result": 4 },
    "json_schema": {
      "type": "object",
      "required": ["result"],
      "properties": {
        "result": { "type": "integer" }
      }
    }
  }
}
```

For non-HTTP contracts, `expect.json` is compared against the observation value,
`expect.json_schema` validates that value, and `expect.effects` compares emitted
side effects.
