#!/usr/bin/env python3
"""Positive и negative controls для source-file-size guard."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import sys


ROOT = Path(__file__).resolve().parent.parent


def load_checker():
    path = ROOT / "scripts/check-source-file-size.py"
    spec = importlib.util.spec_from_file_location("source_file_size_checker", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"module не загружен: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def write_lines(path: Path, count: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("// source line\n" * count, encoding="utf-8")


def main() -> int:
    checker = load_checker()
    with tempfile.TemporaryDirectory(prefix="source-file-size-") as directory:
        fixture = Path(directory)
        write_lines(fixture / "crates/small/src/lib.rs", 1_500)
        write_lines(fixture / "crates/codegen/src/api_generated.rs", 1_700)
        write_lines(fixture / "apps/legacy/src/main.rs", 1_501)

        errors = checker.violations(
            fixture,
            ratchet={"apps/legacy/src/main.rs": 1_501},
        )
        if errors:
            print(f"source file size test: positive fixture rejected: {errors}", file=sys.stderr)
            return 1

        write_lines(fixture / "apps/new-client/src/main.rs", 1_501)
        errors = checker.violations(
            fixture,
            ratchet={"apps/legacy/src/main.rs": 1_501},
        )
        if not any("apps/new-client/src/main.rs" in error for error in errors):
            print("source file size test: oversized file accepted", file=sys.stderr)
            return 1

        write_lines(fixture / "apps/legacy/src/main.rs", 1_502)
        errors = checker.violations(
            fixture,
            ratchet={"apps/legacy/src/main.rs": 1_501},
        )
        if not any("вырос с ratchet 1501" in error for error in errors):
            print("source file size test: ratchet growth accepted", file=sys.stderr)
            return 1

        write_lines(fixture / "apps/legacy/src/main.rs", 1_501)
        errors = checker.violations(
            fixture,
            ratchet={"apps/legacy/src/main.rs": 1_502},
        )
        if not any("снизьте RATCHET" in error for error in errors):
            print("source file size test: ratchet decrease not pinned", file=sys.stderr)
            return 1

        write_lines(fixture / "apps/legacy/src/main.rs", 1_500)
        errors = checker.violations(
            fixture,
            ratchet={"apps/legacy/src/main.rs": 1_501},
        )
        if not any("удалите запись из RATCHET" in error for error in errors):
            print("source file size test: stale ratchet accepted", file=sys.stderr)
            return 1

    print("source file size test: ok (positive=1, negative=4)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
