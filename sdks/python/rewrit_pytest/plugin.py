from __future__ import annotations

import json
import os
import sys
from functools import wraps
from pathlib import Path
from typing import Any, Callable

import pytest

_current_case_id: str | None = None
_current_suite_id: str | None = None
_observed_cases: set[str] = set()


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


def emit_observation(value: Any = None, case_id: str | None = None, status: str = "passed", effects: list[dict[str, Any]] | None = None):
    case_id = case_id or _current_case_id
    if case_id is None:
        raise RuntimeError("Rewrit case id is missing. Use @rewrit_case(...) or pytest.mark.rewrit_case(...).")

    _observed_cases.add(case_id)
    emit_event(
        {
            "schema_version": "rewrit.event.v1",
            "kind": "observation",
            "case_id": case_id,
            "runtime_id": runtime_id(),
            "status": status,
            "value": None if value is None else {"kind": "json", "value": value},
            "error": None,
            "stdout": {"text": "", "truncated": False},
            "stderr": {"text": "", "truncated": False},
            "exit_code": 0,
            "duration_ms": 0,
            "effects": effects or [],
            "artifacts": [],
            "metadata": {} if _current_suite_id is None else {"suite_id": _current_suite_id},
        }
    )


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
