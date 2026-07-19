#!/usr/bin/env python3
"""Positive и negative controls для daemon-client single-home guard."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import sys


ROOT = Path(__file__).resolve().parent.parent


def load_checker():
    path = ROOT / "scripts/check-daemon-client-single-home.py"
    spec = importlib.util.spec_from_file_location("daemon_client_home_checker", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"module не загружен: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def write(path: Path, source: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(source, encoding="utf-8")


def main() -> int:
    checker = load_checker()
    with tempfile.TemporaryDirectory(prefix="daemon-client-home-") as directory:
        fixture = Path(directory)
        write(
            fixture / "apps/telegram-cli/src/main.rs",
            "use telegram_client::DaemonClient;\nfn main() {}\n",
        )
        write(
            fixture / "apps/telegramd/src/socket.rs",
            "fn socket_path() {}\nUnixStream::connect(path);\nUnixListener::bind(path);\n",
        )
        if errors := checker.violations(fixture):
            print(f"daemon client home test: positive fixture rejected: {errors}", file=sys.stderr)
            return 1

        write(
            fixture / "apps/telegram-mcp/src/socket.rs",
            "fn socket_path() {}\nfn validate_socket() {}\nUnixStream::connect(path);\n",
        )
        errors = checker.violations(fixture)
        expected_labels = (
            "определение socket_path/validate_socket",
            "прямой UnixStream::connect",
        )
        if len(errors) != 3 or not all(
            any(label in error for error in errors) for label in expected_labels
        ):
            print(
                f"daemon client home test: duplication not rejected: {errors}",
                file=sys.stderr,
            )
            return 1

    print("daemon client home test: ok (positive=1, negative=3)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
