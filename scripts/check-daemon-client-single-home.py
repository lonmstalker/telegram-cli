#!/usr/bin/env python3
"""Запрещает приложениям дублировать daemon socket client."""

from __future__ import annotations

from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parent.parent
SERVER_APP = "telegramd"
PATTERNS = (
    (
        "определение socket_path/validate_socket",
        re.compile(r"\bfn\s+(?:socket_path|validate_socket)\s*\("),
    ),
    (
        "прямой UnixStream::connect",
        re.compile(r"\bUnixStream\s*::\s*connect\s*\("),
    ),
)


def consumer_sources(root: Path) -> list[Path]:
    apps = root / "apps"
    if not apps.is_dir():
        return []
    return sorted(
        path
        for app in apps.iterdir()
        if app.is_dir() and app.name != SERVER_APP
        for path in app.rglob("*.rs")
    )


def violations(root: Path = ROOT) -> list[str]:
    errors: list[str] = []
    for path in consumer_sources(root):
        for line_number, line in enumerate(
            path.read_text(encoding="utf-8").splitlines(), start=1
        ):
            for label, pattern in PATTERNS:
                if pattern.search(line):
                    relative_path = path.relative_to(root).as_posix()
                    errors.append(f"{relative_path}:{line_number}: {label}")
    return errors


def main() -> int:
    errors = violations()
    if errors:
        for error in errors:
            print(f"daemon client single home: {error}", file=sys.stderr)
        return 1

    print("daemon client single home: ok (home: crates/telegram-client)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
