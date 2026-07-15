#!/usr/bin/env python3
"""Изолированный TDJSON smoke без создания client/DB/background threads."""

from __future__ import annotations

import argparse
import ctypes
import json
import os
from pathlib import Path
import stat
import sys
import tempfile


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--artifact", required=True, type=Path)
    parser.add_argument("--expected-version", required=True)
    parser.add_argument("--expected-commit", required=True)
    parser.add_argument("--output", required=True, type=Path)
    return parser.parse_args()


def execute_option(execute: object, name: str) -> str:
    request = json.dumps(
        {"@type": "getOption", "name": name},
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    raw_result = execute(None, request)
    if raw_result is None:
        raise RuntimeError(f"getOption({name}) returned null")
    try:
        result = json.loads(raw_result.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise RuntimeError(f"getOption({name}) returned invalid JSON") from error
    if result.get("@type") != "optionValueString" or not isinstance(
        result.get("value"), str
    ):
        raise RuntimeError(f"getOption({name}) returned unexpected type")
    return result["value"]


def atomic_write_json(path: Path, value: dict[str, object]) -> None:
    payload = (
        json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n"
    ).encode("utf-8")
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary: Path | None = None
    try:
        descriptor, name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
        temporary = Path(name)
        with os.fdopen(descriptor, "wb") as output:
            output.write(payload)
            output.flush()
            os.fsync(output.fileno())
        temporary.chmod(0o600)
        os.replace(temporary, path)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def main() -> int:
    arguments = parse_arguments()
    artifact = arguments.artifact.resolve(strict=True)
    metadata = artifact.lstat()
    if not stat.S_ISREG(metadata.st_mode):
        raise RuntimeError("artifact must be a regular file without symlink")
    output = arguments.output.resolve()
    initial_entries = {entry.name for entry in Path.cwd().iterdir()}

    library = ctypes.CDLL(str(artifact), mode=os.RTLD_LOCAL | os.RTLD_NOW)
    execute = library.td_json_client_execute
    execute.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
    execute.restype = ctypes.c_char_p
    options = {
        "version": execute_option(execute, "version"),
        "commit_hash": execute_option(execute, "commit_hash"),
    }
    expected_options = {
        "version": arguments.expected_version,
        "commit_hash": arguments.expected_commit,
    }
    if options != expected_options:
        raise RuntimeError("TDJSON version/commit differs from exact native policy")

    created_entries = {
        entry.name for entry in Path.cwd().iterdir()
    } - initial_entries
    if created_entries:
        raise RuntimeError("TDJSON no-client smoke created filesystem entries")
    atomic_write_json(
        output,
        {
            "format_version": 1,
            "options": options,
            "database_files_created": 0,
        },
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError) as error:
        print(f"tdjson smoke: {error}", file=sys.stderr)
        raise SystemExit(1) from error
