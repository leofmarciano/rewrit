import json
import sys
from functools import wraps


def rewrit_case(case_id: str):
    def decorator(fn):
        setattr(fn, "_rewrit_case_id", case_id)

        @wraps(fn)
        def wrapper(*args, **kwargs):
            return fn(*args, **kwargs)

        return wrapper

    return decorator


def emit_observation(case_id: str, runtime_id: str, value=None):
    event = {
        "schema_version": "rewrit.event.v1",
        "kind": "observation",
        "case_id": case_id,
        "runtime_id": runtime_id,
        "status": "passed",
        "value": None if value is None else {"kind": "json", "value": value},
        "error": None,
        "stdout": {"text": "", "truncated": False},
        "stderr": {"text": "", "truncated": False},
        "exit_code": 0,
        "duration_ms": 0,
        "effects": [],
        "artifacts": [],
        "metadata": {},
    }
    sys.stdout.write(json.dumps(event) + "\n")

