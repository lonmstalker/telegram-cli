#!/usr/bin/env python3

from __future__ import annotations

import json
import os
import platform
import subprocess
import sys
from pathlib import Path


CANARY = b"TDLIB_SECRET_CANARY_DO_NOT_LOG"
TEST = "tdjson_native::tests::pinned_native_no_client_call_uses_real_tdjson_transport"


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    if not contains_canary(b"negative-control:" + CANARY):
        print("core secret output: scanner negative control failed", file=sys.stderr)
        return 1

    target = host_target()
    if target is None:
        print("core secret output: ok (native canary unsupported on this host, negative_controls=1)")
        return 0
    provenance_path = root / "vendor" / "tdlib" / "native-builds" / f"{target}.json"
    try:
        provenance = json.loads(provenance_path.read_text(encoding="utf-8"))
        library = root / provenance["artifact"]["cache_path"]
    except (OSError, KeyError, json.JSONDecodeError):
        print("core secret output: invalid native provenance", file=sys.stderr)
        return 1
    if not library.is_file():
        print("core secret output: ok (local native artifact absent, negative_controls=1)")
        return 0

    environment = os.environ.copy()
    environment["TDJSON_LIBRARY_PATH"] = str(library)
    try:
        result = subprocess.run(
            [
                "cargo",
                "test",
                "-p",
                "telegram-core",
                TEST,
                "--",
                "--ignored",
                "--exact",
            ],
            cwd=root,
            env=environment,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=30,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        print("core secret output: native canary execution failed", file=sys.stderr)
        return 1
    if result.returncode != 0:
        print("core secret output: native canary test failed", file=sys.stderr)
        return 1
    if contains_canary(result.stdout):
        print("core secret output: canary leaked to process output", file=sys.stderr)
        return 1
    print("core secret output: ok (native canary clean, negative_controls=1)")
    return 0


def host_target() -> str | None:
    system = platform.system()
    machine = platform.machine().lower()
    if system == "Darwin" and machine in {"arm64", "aarch64"}:
        return "aarch64-apple-darwin"
    if system == "Linux" and machine in {"x86_64", "amd64"}:
        return "x86_64-unknown-linux-gnu"
    return None


def contains_canary(output: bytes) -> bool:
    return CANARY in output


if __name__ == "__main__":
    raise SystemExit(main())
