from __future__ import annotations

import json
import os
import sys
from pathlib import Path


RUNTIME_ID = os.environ.get("REWRIT_RUNTIME_ID", "reference_django")
COMMAND = os.environ.get("REWRIT_ADAPTER_COMMAND", "run")
CASE_ID = "billing.invoice.create.success"


def emit(event: dict) -> None:
    encoded = json.dumps(event, separators=(",", ":")) + "\n"
    events_path = os.environ.get("REWRIT_EVENTS_PATH")
    if events_path:
        Path(events_path).parent.mkdir(parents=True, exist_ok=True)
        with open(events_path, "a", encoding="utf-8") as handle:
            handle.write(encoded)
        return
    sys.stdout.write(encoded)


def text(value: str) -> dict:
    return {"kind": "string", "value": value}


def case_discovered() -> None:
    emit(
        {
            "schema_version": "rewrit.event.v1",
            "kind": "case_discovered",
            "runtime_id": RUNTIME_ID,
            "case": {
                "id": CASE_ID,
                "suite_id": "billing",
                "title": "creates invoice",
                "source_location": None,
                "tags": [],
                "contract_ref": None,
                "required": True,
            },
        }
    )


if COMMAND == "doctor":
    emit(
        {
            "schema_version": "rewrit.event.v1",
            "kind": "doctor_report",
            "runtime_id": RUNTIME_ID,
            "report": {"ok": True, "checks": {"python": sys.version.split()[0], "django": "fixture"}},
        }
    )
    raise SystemExit(0)

case_discovered()

if COMMAND == "discover":
    raise SystemExit(0)

emit(
    {
        "schema_version": "rewrit.event.v1",
        "kind": "observation",
        "case_id": CASE_ID,
        "runtime_id": RUNTIME_ID,
        "status": "passed",
        "value": {
            "kind": "object",
            "fields": {
                "status": {"kind": "integer", "value": "201"},
                "headers": {
                    "kind": "object",
                    "fields": {
                        "content-type": text("application/json"),
                        "x-request-id": text("django-request-id"),
                    },
                },
                "body": {
                    "kind": "json",
                    "value": {
                        "id": "inv_123",
                        "amount": "199.90",
                        "currency": "BRL",
                        "status": "open",
                    },
                },
            },
        },
        "error": None,
        "stdout": {"text": "", "truncated": False},
        "stderr": {"text": "", "truncated": False},
        "exit_code": 0,
        "duration_ms": 1,
        "effects": [],
        "artifacts": [],
        "metadata": {"suite_id": "billing"},
    }
)
