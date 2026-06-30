# Adapter Protocol v1

Adapters communicate with the engine using newline-delimited JSON events.

Every message includes `schema_version`.

```json
{"schema_version":"rewrit.event.v1","kind":"case_started","case_id":"billing.invoice.create.success","runtime_id":"reference"}
{"schema_version":"rewrit.event.v1","kind":"observation","case_id":"billing.invoice.create.success","runtime_id":"reference","status":"passed","value":{"kind":"json","value":{"status":"open"}},"error":null,"stdout":{"text":"","truncated":false},"stderr":{"text":"","truncated":false},"exit_code":0,"duration_ms":10,"effects":[],"artifacts":[],"metadata":{}}
```

