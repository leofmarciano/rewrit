from __future__ import annotations

import json
import os
import sys
from base64 import b64encode
from collections.abc import Mapping
from datetime import date, datetime
from decimal import Decimal
from functools import wraps
from pathlib import Path
from typing import Any, Callable

import pytest

_current_case_id: str | None = None
_current_suite_id: str | None = None
_observed_cases: set[str] = set()
_last_observation: dict[str, Any] | None = None


def rewrit_case(case_id: str, suite_id: str | None = None, title: str | None = None):
    def decorator(fn: Callable[..., Any]):
        setattr(fn, "_rewrit_case_id", case_id)
        setattr(fn, "_rewrit_suite_id", suite_id)
        setattr(fn, "_rewrit_title", title)

        @wraps(fn)
        def wrapper(*args: Any, **kwargs: Any):
            return fn(*args, **kwargs)

        return wrapper

    return decorator


def emit_observation(
    value: Any = None,
    case_id: str | None = None,
    status: str = "passed",
    effects: list[dict[str, Any]] | None = None,
):
    emit_canonical_observation(
        None if value is None else {"kind": "json", "value": value},
        case_id,
        status,
        effects,
    )


def emit_canonical_observation(
    value: dict[str, Any] | None = None,
    case_id: str | None = None,
    status: str = "passed",
    effects: list[dict[str, Any]] | None = None,
):
    global _last_observation
    case_id = case_id or _current_case_id
    if case_id is None:
        raise RuntimeError("Rewrit case id is missing. Use @rewrit_case(...) or pytest.mark.rewrit_case(...).")

    event = observation_event(case_id, value, status, effects or [])
    _observed_cases.add(case_id)
    _last_observation = event
    emit_event(event)


def add_effect(effect: dict[str, Any], case_id: str | None = None):
    global _last_observation
    case_id = case_id or _current_case_id
    if case_id is None:
        raise RuntimeError("Rewrit case id is missing. Use @rewrit_case(...) or pytest.mark.rewrit_case(...).")

    if _last_observation is not None and _last_observation.get("case_id") == case_id:
        effects = list(_last_observation.get("effects", []))
        effects.append(effect)
        _last_observation = {**_last_observation, "effects": effects}
        emit_event(_last_observation)
        return

    emit_canonical_observation(None, case_id, "passed", [effect])


def observation_event(case_id: str, value: dict[str, Any] | None, status: str, effects: list[dict[str, Any]]):
    return {
        "schema_version": "rewrit.event.v1",
        "kind": "observation",
        "case_id": case_id,
        "runtime_id": runtime_id(),
        "status": status,
        "value": value,
        "error": None,
        "stdout": {"text": "", "truncated": False},
        "stderr": {"text": "", "truncated": False},
        "exit_code": 0,
        "duration_ms": 0,
        "effects": effects,
        "artifacts": [],
        "metadata": {} if _current_suite_id is None else {"suite_id": _current_suite_id},
    }


def emit_case_discovered(case_id: str, suite_id: str | None = None, title: str | None = None, source_path: str | None = None, line: int | None = None):
    emit_event(
        {
            "schema_version": "rewrit.event.v1",
            "kind": "case_discovered",
            "runtime_id": runtime_id(),
            "case": {
                "id": case_id,
                "suite_id": suite_id or suite_from_case_id(case_id),
                "title": title or case_id,
                "source_location": None if source_path is None else {"path": source_path, "line": line, "column": None},
                "tags": [],
                "contract_ref": None,
                "required": True,
            },
        }
    )


def emit_event(event: dict[str, Any]):
    encoded = json.dumps(event, separators=(",", ":")) + "\n"
    events_path = os.environ.get("REWRIT_EVENTS_PATH")
    if events_path:
        Path(events_path).parent.mkdir(parents=True, exist_ok=True)
        with open(events_path, "a", encoding="utf-8") as handle:
            handle.write(encoded)
        return

    sys.stdout.write(encoded)


def runtime_id() -> str:
    return os.environ.get("REWRIT_RUNTIME_ID", "reference")


def canonical_value(value: Any) -> dict[str, Any]:
    if value is None:
        return {"kind": "null"}
    if isinstance(value, bool):
        return {"kind": "bool", "value": value}
    if isinstance(value, int):
        return {"kind": "integer", "value": str(value)}
    if isinstance(value, Decimal):
        return {"kind": "decimal", "value": str(value)}
    if isinstance(value, float):
        return {"kind": "float", "value": str(value)}
    if isinstance(value, str):
        return {"kind": "string", "value": value}
    if isinstance(value, bytes | bytearray):
        return {"kind": "bytes", "base64": b64encode(value).decode("ascii"), "media_type": None}
    if isinstance(value, datetime):
        return {"kind": "date_time", "rfc3339": value.isoformat()}
    if isinstance(value, date):
        return {"kind": "string", "value": value.isoformat()}
    if isinstance(value, Mapping):
        return {"kind": "object", "fields": {str(key): canonical_value(entry) for key, entry in value.items()}}
    if isinstance(value, list | tuple):
        return {"kind": "array", "items": [canonical_value(entry) for entry in value]}
    return {"kind": "string", "value": str(value)}


def pytest_configure(config: pytest.Config):
    config.addinivalue_line("markers", "rewrit_case(case_id, suite_id=None, title=None): mark a test as a Rewrit case")


def pytest_collection_modifyitems(config: pytest.Config, items: list[pytest.Item]):
    for item in items:
        case = case_for_item(item)
        if case is None:
            continue
        case_id, suite_id, title = case
        emit_case_discovered(case_id, suite_id, title or item.name, str(item.path), getattr(item, "lineno", None))


def pytest_runtest_setup(item: pytest.Item):
    global _current_case_id, _current_suite_id
    case = case_for_item(item)
    if case is None:
        _current_case_id = None
        _current_suite_id = None
        return
    _current_case_id, _current_suite_id, _title = case


@pytest.hookimpl(hookwrapper=True)
def pytest_runtest_makereport(item: pytest.Item, call: pytest.CallInfo[Any]):
    outcome = yield
    report = outcome.get_result()
    if report.when != "call":
        return
    case = case_for_item(item)
    if case is None:
        return
    case_id, _suite_id, _title = case
    if case_id in _observed_cases:
        return
    emit_observation(None, case_id, "passed" if report.passed else "failed")


def case_for_item(item: pytest.Item) -> tuple[str, str | None, str | None] | None:
    marker = item.get_closest_marker("rewrit_case")
    if marker is not None and marker.args:
        return marker.args[0], marker.kwargs.get("suite_id"), marker.kwargs.get("title")

    obj = getattr(item, "obj", None)
    case_id = getattr(obj, "_rewrit_case_id", None)
    if case_id is None:
        return None
    return case_id, getattr(obj, "_rewrit_suite_id", None), getattr(obj, "_rewrit_title", None)


def suite_from_case_id(case_id: str) -> str:
    return case_id.split(".", 1)[0] if "." in case_id else "default"
