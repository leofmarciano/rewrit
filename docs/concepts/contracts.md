# Contracts

Contracts describe behavior that must remain equivalent across the reference
and candidate implementations.

They are versioned documents with stable case IDs. A contract can describe an
HTTP request, a command, a job, a function call, or another boundary that an
adapter knows how to execute.

## Shape

Current contracts use `schema_version = "rewrit.contract.v1"` and this core
shape:

```json
{
  "schema_version": "rewrit.contract.v1",
  "id": "billing.invoice.create.success",
  "kind": "http_case",
  "input": {},
  "expect": {},
  "policy": "http_api_strict",
  "metadata": {}
}
```

Important fields:

- `id`: stable case ID used to join contracts, observations, reports, and
  waivers.
- `kind`: adapter-facing case type such as `http_case`, `command_case`,
  `job_case`, or `function_case`.
- `input`: method, path, headers, JSON body, or raw body for runtimes that
  execute from contracts.
- `expect`: expected status, headers, JSON value, JSON schema, and side effects.
- `policy`: optional policy name from `rewrit.toml`.
- `metadata`: string key/value metadata for reporting and tooling.

## HTTP Example

```json
{
  "schema_version": "rewrit.contract.v1",
  "id": "billing.invoice.create.success",
  "kind": "http_case",
  "input": {
    "method": "POST",
    "path": "/api/invoices",
    "headers": {
      "content-type": "application/json"
    },
    "json": {
      "customer_id": "cus_123",
      "amount": "199.90",
      "currency": "BRL"
    }
  },
  "expect": {
    "status": 201,
    "json_schema": {
      "type": "object",
      "required": ["id", "amount", "currency", "status"],
      "properties": {
        "id": { "type": "string" },
        "amount": { "type": "string", "pattern": "^\\d+\\.\\d{2}$" },
        "currency": { "const": "BRL" },
        "status": { "const": "open" }
      }
    },
    "effects": [
      {
        "kind": "db_delta",
        "connection": "default",
        "table": "invoices",
        "inserted": [
          {
            "customer_id": { "kind": "string", "value": "cus_123" },
            "amount": { "kind": "string", "value": "199.90" }
          }
        ],
        "updated": [],
        "deleted": []
      }
    ]
  },
  "policy": "http_api_strict"
}
```

## Non-HTTP Contracts

For non-HTTP contracts, Rewrit passes selected case IDs to command-compatible
adapters through:

- the adapter request file when `[runtimes.<id>.protocol].input = "file"`,
- the runtime environment managed by the engine.

The adapter is responsible for executing the matching command, job, test, or
function and emitting observations with the same `case_id`.

Example:

```json
{
  "schema_version": "rewrit.contract.v1",
  "id": "catalog.product.price.success",
  "kind": "function_case",
  "input": {
    "json": { "product_id": "sku_123" }
  },
  "expect": {
    "json": {
      "price": "99.90",
      "currency": "BRL"
    }
  }
}
```

## Authoring Rules

- Keep IDs stable across implementations.
- Prefer domain language over framework names.
- Declare money as strings or canonical decimals, not floats.
- Put volatile fields behind scoped normalizers instead of removing the whole
  comparison.
- Use JSON schema for response shape and `expect.json` for exact values.
- Add side effects only when they are part of the behavior contract.
- Keep contracts small enough that a failed case points to one behavior.

## Validation

Generate the current contract schema:

```bash
cargo run -p rewrit-cli -- schema export --kind contract
```

Generate all protocol and report schemas:

```bash
cargo run -p rewrit-cli -- schema export --kind all --out-dir dist/schemas
```
