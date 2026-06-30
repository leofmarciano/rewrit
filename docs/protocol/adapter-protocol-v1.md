# Adapter Protocol v1

Adapters connect framework-specific execution to the Rewrit engine.

The protocol is newline-delimited JSON. Every line is one complete event. Every
event includes `schema_version`.

## Why NDJSON?

NDJSON is intentionally boring:

- it streams,
- it works in every language,
- it is easy to inspect with a terminal,
- it avoids FFI,
- it avoids long-running adapter servers,
- it works for large monoliths and simple scripts.

The engine treats stdout or an events file as the protocol stream. Human test
runner output should not be mixed into protocol stdout unless the runner only
prints Rewrit events.

## Runtime Configuration

The simplest adapter is a command that emits Rewrit events:

```toml
[runtimes.reference]
adapter = "command"
cwd = "../legacy"
command = ["vendor/bin/pest", "--rewrit"]
timeout_ms = 30000
```

By default, Rewrit reads protocol events from process stdout.

For noisy runners, use file output:

```toml
[runtimes.reference.protocol]
output = "file"
```

When `output = "file"`, the engine sets `REWRIT_EVENTS_PATH` and the adapter
must append event lines to that file.

Adapters that need a request file can enable file input:

```toml
[runtimes.reference.protocol]
input = "file"
output = "file"
```

When `input = "file"`, the engine writes one adapter request line and exposes
its path as `REWRIT_REQUEST_PATH`.

## Environment Variables

The command runner sets:

```txt
REWRIT_RUNTIME_ID
REWRIT_ADAPTER_COMMAND
REWRIT_PROTOCOL_INPUT
REWRIT_PROTOCOL_OUTPUT
REWRIT_EVENTS_PATH      # when output = "file"
REWRIT_REQUEST_PATH     # when input = "file"
REWRIT_NETWORK_MODE
```

The engine also applies runtime-local variables from
`[runtimes.<id>.env]`.

## Adapter Requests

An adapter request tells the runtime what the engine wants it to do.

```json
{"schema_version":"rewrit.adapter_request.v1","command":"run","runtime_id":"reference","cases":[]}
```

Commands:

- `doctor`: report whether the adapter can run.
- `discover`: emit available cases without running them.
- `run`: execute cases and emit observations.

When `cases` is empty, the adapter should run all applicable cases for the
current command.

## Event Types

### Doctor Report

Use this for adapter self-checks:

```json
{"schema_version":"rewrit.event.v1","kind":"doctor_report","runtime_id":"reference","report":{"ok":true,"checks":{"php":"8.3","pest":"loaded"}}}
```

### Case Discovered

Use this when a runner can enumerate cases:

```json
{"schema_version":"rewrit.event.v1","kind":"case_discovered","runtime_id":"reference","case":{"id":"billing.invoice.create.success","suite_id":"billing","title":"creates invoice","source_location":{"path":"tests/BillingTest.php","line":42,"column":null},"tags":[],"contract_ref":null,"required":true}}
```

Discovery is how `rewrit audit` catches missing candidate coverage.

### Case Started and Finished

These events are optional but useful for long-running adapters:

```json
{"schema_version":"rewrit.event.v1","kind":"case_started","case_id":"billing.invoice.create.success","runtime_id":"reference"}
{"schema_version":"rewrit.event.v1","kind":"case_finished","case_id":"billing.invoice.create.success","runtime_id":"reference","duration_ms":82}
```

### Observation

This is the main event:

```json
{"schema_version":"rewrit.event.v1","kind":"observation","case_id":"billing.invoice.create.success","runtime_id":"reference","status":"passed","value":{"kind":"json","value":{"status":"open","amount":"199.90"}},"error":null,"stdout":{"text":"","truncated":false},"stderr":{"text":"","truncated":false},"exit_code":0,"duration_ms":10,"effects":[],"artifacts":[],"metadata":{"suite_id":"billing"}}
```

See [Observation schema v1](observation-schema-v1.md) for field details.

### Adapter Error

Use this when the adapter itself fails before it can produce a meaningful case
observation:

```json
{"schema_version":"rewrit.event.v1","kind":"adapter_error","runtime_id":"reference","case_id":null,"message":"pytest plugin is not installed","retryable":false}
```

Do not use `adapter_error` for real business mismatches. Those should become
observations and divergences.

## Output Rules

- Emit one JSON object per line.
- Do not pretty-print events in the protocol stream.
- Use `schema_version = "rewrit.event.v1"` for events.
- Use stable `case_id` values across reference and candidate.
- Prefer file output when the underlying runner writes human text to stdout.
- Keep events deterministic; put volatile values behind normalizers.
- Make adapter failures explicit instead of hiding them as skipped cases.

## Schemas

Generate all current schemas locally:

```bash
cargo run -p rewrit-cli -- schema export --kind all --out-dir dist/schemas
```

Generated files:

```txt
rewrit.adapter_request.v1.schema.json
rewrit.contract.v1.schema.json
rewrit.event.v1.schema.json
rewrit.observation.v1.schema.json
rewrit.report.v1.schema.json
```
