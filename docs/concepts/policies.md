# Policies

Policies decide which differences are meaningful after observations have been
normalized.

Rewrit is strict by default. Tolerance must be explicit, scoped, and visible in
reports.

## Default Behavior

The default policy:

- compares canonical values,
- compares exit code,
- ignores stdout and stderr unless enabled,
- ignores duration unless enabled,
- treats `null` and `absent` as different,
- treats integer/float equivalence as false,
- treats object key order as irrelevant,
- treats HTTP header names case-insensitively,
- keeps money/string/decimal/float differences visible,
- ignores common noisy headers such as `date`, `server`, and `x-request-id`.

## Manifest Example

```toml
[policies.http_api_strict]
compare_exit_code = true
compare_stdout = false
compare_stderr = false
allow_null_absent_equivalence = false
allow_integer_float_equivalence = false
decimal_as_string = true

[policies.http_api_strict.headers]
ignore = ["date", "x-request-id", "server"]

[policies.http_api_strict.json]
ignore_paths = ["$.body.generated_at", "$.body.trace_id"]
unordered_paths = ["$.body.items[*].metadata"]
```

Current implementation note: the engine applies the first policy declared in
`rewrit.toml` as the effective run policy. Suite- or case-specific policy
selection is part of the manifest shape but should not be relied on until the
selection behavior is implemented.

## Normalizers

Normalizers rewrite known noise before comparison. They do not hide that they
ran; applied normalizers are included in reports.

Supported normalizer kinds:

```toml
[[normalizers]]
kind = "uuid"
paths = ["$.body.id"]

[[normalizers]]
kind = "timestamp"
paths = ["$.body.generated_at"]

[[normalizers]]
kind = "regex"
pattern = "\\breq_[A-Za-z0-9]+\\b"
replacement = "<REQUEST_ID>"
paths = ["$.body.trace_id"]

[[normalizers]]
kind = "path"
replace_project_root = "<PROJECT_ROOT>"

[[normalizers]]
kind = "http_headers"
```

Prefer path-scoped normalizers. A global regex normalizer can remove useful
evidence from unrelated fields, stdout, stderr, or errors.

## Waivers

Waivers allow known divergences for a limited time.

```toml
[[waivers]]
case = "billing.invoice.cancel.refund_event"
kind = "side_effect_mismatch"
reason = "Encore does not publish RefundIssued yet"
owner = "billing-platform"
expires = "2026-08-01"
issue = "BILL-4821"
```

Rules:

- A waiver matches a specific `case` and divergence `kind`.
- Non-expired waivers change severity to `allowed`.
- Expired waivers become blocking `waiver_expired` divergences.
- Waived differences remain in reports.

## Choosing Tolerance

Use a normalizer when both implementations are equivalent but contain accepted
noise, such as generated IDs or timestamps.

Use a policy option when a whole class of difference is acceptable for the run,
such as ignoring stdout.

Use a waiver when the candidate is knowingly incomplete and the team wants a
temporary, owned exception.
