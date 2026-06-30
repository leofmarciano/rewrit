# Adapter Protocol v1

Adapters communicate with the engine using newline-delimited JSON events.

Every message includes `schema_version`.

```json
{"schema_version":"rewrit.event.v1","kind":"case_started","case_id":"billing.invoice.create.success","runtime_id":"reference"}
{"schema_version":"rewrit.event.v1","kind":"observation","case_id":"billing.invoice.create.success","runtime_id":"reference","status":"passed","value":{"kind":"json","value":{"status":"open"}},"error":null,"stdout":{"text":"","truncated":false},"stderr":{"text":"","truncated":false},"exit_code":0,"duration_ms":10,"effects":[],"artifacts":[],"metadata":{}}
```

## Command transport

The default command adapter transport reads events from process stdout.

```toml
[runtimes.reference]
adapter = "command"
command = ["vendor/bin/pest", "--rewrit"]
```

Large adapters can use file transport instead:

```toml
[runtimes.reference.protocol]
input = "file"
output = "file"
```

When `input = "file"`, Rewrit writes one NDJSON request line and exposes its
path as `REWRIT_REQUEST_PATH`.

```json
{"schema_version":"rewrit.adapter_request.v1","command":"run","runtime_id":"reference","cases":[]}
```

When `output = "file"`, the adapter must write event NDJSON to
`REWRIT_EVENTS_PATH`. Rewrit also sets:

```txt
REWRIT_RUNTIME_ID
REWRIT_ADAPTER_COMMAND
REWRIT_PROTOCOL_INPUT
REWRIT_PROTOCOL_OUTPUT
```

## Schemas

Generate all current schemas locally with:

```bash
rewrit schema export --kind all --out-dir dist/schemas
```

Release workflows publish these files as the `rewrit-protocol-schemas` artifact:

```txt
rewrit.adapter_request.v1.schema.json
rewrit.contract.v1.schema.json
rewrit.event.v1.schema.json
rewrit.observation.v1.schema.json
rewrit.report.v1.schema.json
```
