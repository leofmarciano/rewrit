#!/usr/bin/env sh
set -eu

printf '%s\n' '{"schema_version":"rewrit.event.v1","kind":"case_discovered","runtime_id":"reference","case":{"id":"billing.invoice.create.success","suite_id":"billing","title":"creates invoice","source_location":null,"tags":[],"contract_ref":null,"required":true}}'
printf '%s\n' '{"schema_version":"rewrit.event.v1","kind":"observation","case_id":"billing.invoice.create.success","runtime_id":"reference","status":"passed","value":{"kind":"json","value":{"id":"inv_123","amount":"199.90","currency":"BRL","status":"open"}},"error":null,"stdout":{"text":"","truncated":false},"stderr":{"text":"","truncated":false},"exit_code":0,"duration_ms":12,"effects":[],"artifacts":[],"metadata":{}}'
printf '%s\n' '{"schema_version":"rewrit.event.v1","kind":"observation","case_id":"auth.login.invalid_password","runtime_id":"reference","status":"failed","value":null,"error":{"kind":"validation","code":"INVALID_CREDENTIALS","class":null,"message":"Invalid credentials","normalized_message":"Invalid credentials","http_status":422,"retryable":false,"frames":[]},"stdout":{"text":"","truncated":false},"stderr":{"text":"","truncated":false},"exit_code":0,"duration_ms":8,"effects":[],"artifacts":[],"metadata":{}}'

