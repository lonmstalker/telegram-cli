#!/usr/bin/env python3
"""Не даёт entrypoints без конфигурации имитировать успешный runtime."""

from __future__ import annotations

import os
from pathlib import Path
import signal
import subprocess
import sys
import time
from typing import NamedTuple


ROOT = Path(__file__).resolve().parent.parent
BINARY_MESSAGES = {
    "telegram-cli": "telegram-cli: usage: telegram-cli session ... | login [tty] | schema ... | td call <lease_id> <json> | workflow list|describe|run ... | events watch ...",
    "telegram-mcp": "telegram-mcp: runtime ещё не реализован",
    "telegram-webapp-runner": "telegram-webapp-runner: runtime ещё не реализован",
    "telegramd": "telegramd: runtime ещё не реализован",
}


class CommandResult(NamedTuple):
    pid: int
    returncode: int
    stdout: str
    stderr: str
    timed_out: bool


def signal_process_group(process: subprocess.Popen[str], signal_number: int) -> None:
    try:
        os.killpg(process.pid, signal_number)
    except (PermissionError, ProcessLookupError):
        pass


def process_group_exists(process: subprocess.Popen[str]) -> bool:
    process.poll()
    try:
        os.killpg(process.pid, 0)
    except (PermissionError, ProcessLookupError):
        return False
    return True


def stop_process_group(
    process: subprocess.Popen[str], terminate_grace_seconds: float
) -> None:
    if process_group_exists(process):
        signal_process_group(process, signal.SIGTERM)
        deadline = time.monotonic() + terminate_grace_seconds
        while process_group_exists(process) and time.monotonic() < deadline:
            process.poll()
            time.sleep(0.005)

    if process_group_exists(process):
        signal_process_group(process, signal.SIGKILL)

    try:
        process.wait(timeout=max(terminate_grace_seconds, 0.1))
    except subprocess.TimeoutExpired:
        signal_process_group(process, signal.SIGKILL)
        process.wait()

    deadline = time.monotonic() + max(terminate_grace_seconds, 0.1)
    while process_group_exists(process) and time.monotonic() < deadline:
        time.sleep(0.005)
    if process_group_exists(process):
        signal_process_group(process, signal.SIGKILL)


def run_bounded(
    command: list[str],
    *,
    environment: dict[str, str],
    timeout_seconds: float,
    terminate_grace_seconds: float = 1.0,
) -> CommandResult:
    process = subprocess.Popen(
        command,
        cwd=ROOT,
        env=environment,
        start_new_session=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    timed_out = False
    stdout = ""
    stderr = ""
    try:
        stdout, stderr = process.communicate(timeout=timeout_seconds)
    except subprocess.TimeoutExpired:
        timed_out = True
    finally:
        stop_process_group(process, terminate_grace_seconds)

    if timed_out:
        stdout, stderr = process.communicate()

    returncode = process.wait()
    return CommandResult(process.pid, returncode, stdout, stderr, timed_out)


def main() -> int:
    environment = os.environ.copy()
    environment["CARGO_BUILD_JOBS"] = "2"
    environment["CARGO_INCREMENTAL"] = "0"
    environment.pop("TELEGRAM_PROFILE", None)
    environment.pop("TDLIB_DATABASE_DIR", None)
    errors: list[str] = []

    for package, expected_message in BINARY_MESSAGES.items():
        result = run_bounded(
            ["cargo", "run", "--quiet", "--locked", "--package", package],
            environment=environment,
            timeout_seconds=10.0,
        )
        if result.timed_out:
            errors.append(f"{package}: запуск превысил timeout и был остановлен")
        if result.returncode == 0:
            errors.append(f"{package}: незаполненный binary вернул exit code 0")
        if result.stderr.strip() != expected_message:
            errors.append(f"{package}: отсутствует точное fail-closed сообщение")

    if errors:
        for error in errors:
            print(f"skeleton contract: {error}", file=sys.stderr)
        return 1

    print("skeleton contract: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
