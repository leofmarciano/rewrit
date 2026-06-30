#!/usr/bin/env sh
set -eu

printf '%s\n' '{"schema_version":"rewrit.event.v1","kind":"case_discovered","runtime_id":"candidate","case":{"id":"billing.invoice.create.success","suite_id":"billing","title":"creates invoice","source_location":null,"tags":[],"contract_ref":null,"required":true}}'
printf '%s\n' '{"schema_version":"rewrit.event.v1","kind":"observation","case_id":"billing.invoice.create.success","runtime_id":"candidate","status":"passed","value":{"kind":"json","value":{"id":"inv_123","amount":199.9,"currency":"BRL","status":"open"}},"error":null,"stdout":{"text":"","truncated":false},"stderr":{"text":"","truncated":false},"exit_code":0,"duration_ms":9,"effects":[],"artifacts":[],"metadata":{}}'

