from __future__ import annotations

import json
from collections.abc import Mapping
from typing import Any

from .plugin import add_effect, canonical_value, emit_canonical_observation


def observe_http_response(
    response: Any,
    case_id: str | None = None,
    effects: list[dict[str, Any]] | None = None,
):
    status = status_code(response)
    emit_canonical_observation(
        {
            "kind": "object",
            "fields": {
                "status": {"kind": "integer", "value": str(status)},
                "headers": {"kind": "object", "fields": canonical_headers(headers(response))},
                "body": body(response),
            },
        },
        case_id,
        "passed" if status < 500 else "failed",
        effects or [],
    )


def observe_db_delta(
    table: str,
    inserted: list[dict[str, Any]] | None = None,
    updated: list[dict[str, Any]] | None = None,
    deleted: list[dict[str, Any]] | None = None,
    connection: str = "default",
    case_id: str | None = None,
):
    add_effect(db_delta(table, inserted, updated, deleted, connection), case_id)


def db_delta(
    table: str,
    inserted: list[dict[str, Any]] | None = None,
    updated: list[dict[str, Any]] | None = None,
    deleted: list[dict[str, Any]] | None = None,
    connection: str = "default",
) -> dict[str, Any]:
    return {
        "kind": "db_delta",
        "connection": connection,
        "table": table,
        "inserted": [canonical_row(row) for row in inserted or []],
        "updated": [canonical_row(row) for row in updated or []],
        "deleted": [canonical_row(row) for row in deleted or []],
    }


def status_code(response: Any) -> int:
    return int(getattr(response, "status_code", 0) or 0)


def headers(response: Any) -> dict[str, str]:
    source = getattr(response, "headers", None)
    if isinstance(source, Mapping):
        return {str(name).lower(): header_value(value) for name, value in source.items()}

    if hasattr(response, "items"):
        return {str(name).lower(): header_value(value) for name, value in response.items()}

    return {}


def canonical_headers(values: dict[str, str]) -> dict[str, dict[str, Any]]:
    return {name: {"kind": "string", "value": value} for name, value in values.items()}


def body(response: Any) -> dict[str, Any]:
    json_method = getattr(response, "json", None)
    if callable(json_method):
        try:
            return {"kind": "json", "value": json_method()}
        except ValueError:
            pass

    content = getattr(response, "content", None)
    if isinstance(content, bytes | bytearray):
        text = content.decode(getattr(response, "charset", None) or "utf-8", errors="replace")
    elif content is not None:
        text = str(content)
    else:
        text = str(getattr(response, "text", ""))

    try:
        return {"kind": "json", "value": json.loads(text)}
    except ValueError:
        return {"kind": "string", "value": text}


def header_value(value: Any) -> str:
    if isinstance(value, list | tuple):
        return ", ".join(str(entry) for entry in value)
    return str(value)


def canonical_row(row: dict[str, Any]) -> dict[str, Any]:
    return {str(field): canonical_value(value) for field, value in row.items()}
