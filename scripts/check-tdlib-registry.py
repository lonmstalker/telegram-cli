#!/usr/bin/env python3
"""Проверяет generated Rust registry и coverage report pinned schema."""

from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parent.parent


def main() -> int:
    result = subprocess.run(
        ["cargo", "run", "--quiet", "-p", "tdlib-registry-gen", "--", "--check"],
        cwd=ROOT,
        check=False,
    )
    if result.returncode:
        print("generated TDLib registry/report gate: failed", file=sys.stderr)
        return result.returncode
    print("generated TDLib registry/report gate: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
