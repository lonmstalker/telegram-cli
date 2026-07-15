#!/usr/bin/env python3
"""Доказывает cleanup process group после SIGKILL Python guard-parent."""

from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import signal
import stat
import subprocess
import sys
import tempfile
import time


ROOT = Path(__file__).resolve().parent.parent
BUILDER_PATH = ROOT / "scripts/build-tdlib-native.py"


def load_builder():
    spec = importlib.util.spec_from_file_location("tdlib_native_builder", BUILDER_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("native builder module не загружен")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def process_exists(process_id: int) -> bool:
    try:
        os.kill(process_id, 0)
    except ProcessLookupError:
        return False
    return True


def group_exists(group_id: int) -> bool:
    result = subprocess.run(
        ["/bin/ps", "-axo", "pgid="],
        check=True,
        capture_output=True,
        text=True,
        timeout=5,
    )
    return any(line.strip() == str(group_id) for line in result.stdout.splitlines())


def safe_kill_group(group_id: int) -> None:
    if group_id <= 1 or group_id == os.getpgrp():
        return
    try:
        os.killpg(group_id, signal.SIGKILL)
    except (ProcessLookupError, PermissionError):
        pass


def main() -> int:
    builder = load_builder()
    if not hasattr(builder, "guard_state_path"):
        print("tdlib parent-death test: guard_state_path отсутствует", file=sys.stderr)
        return 1

    helper: subprocess.Popen[bytes] | None = None
    target_group = -1
    watchdog_process = -1
    with tempfile.TemporaryDirectory(prefix="tdlib-parent-death-") as directory:
        root = Path(directory)
        log_path = root / "guard.log"
        state_path = builder.guard_state_path(log_path)
        helper_code = (
            "import os,sys; "
            f"sys.path.insert(0,{str(ROOT / 'scripts')!r}); "
            "from tdlib_native import run_guarded; "
            "from pathlib import Path; "
            "run_guarded("
            "[sys.executable,'-c','import time;time.sleep(60)'],"
            f"cwd=Path({str(root)!r}),"
            "environment={'PATH':os.environ['PATH']},"
            f"log_path=Path({str(log_path)!r}),"
            "timeout_seconds=60,maximum_tree_bytes=1048576,"
            "maximum_group_rss_bytes=268435456,maximum_log_bytes=1048576,"
            "poll_seconds=0.05)"
        )
        try:
            helper = subprocess.Popen(
                [sys.executable, "-c", helper_code],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                close_fds=True,
                start_new_session=True,
            )
            deadline = time.monotonic() + 10
            while not state_path.exists() and time.monotonic() < deadline:
                if helper.poll() is not None:
                    break
                time.sleep(0.02)
            if not state_path.exists():
                print("tdlib parent-death test: watchdog state не создан", file=sys.stderr)
                return 1
            metadata = state_path.lstat()
            if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > 4096:
                print("tdlib parent-death test: watchdog state unsafe", file=sys.stderr)
                return 1
            state = json.loads(state_path.read_text(encoding="utf-8"))
            target_group = state["target_process_group_id"]
            watchdog_process = state["watchdog_process_id"]
            if (
                state["guard_parent_process_id"] != helper.pid
                or target_group <= 1
                or watchdog_process <= 1
            ):
                print("tdlib parent-death test: watchdog state invalid", file=sys.stderr)
                return 1

            os.kill(helper.pid, signal.SIGKILL)
            helper.wait(timeout=5)
            cleanup_deadline = time.monotonic() + 10
            while time.monotonic() < cleanup_deadline:
                if (
                    not group_exists(target_group)
                    and not process_exists(watchdog_process)
                    and not state_path.exists()
                ):
                    break
                time.sleep(0.05)
            if group_exists(target_group):
                print("tdlib parent-death test: target group осталась", file=sys.stderr)
                return 1
            if process_exists(watchdog_process):
                print("tdlib parent-death test: watchdog остался", file=sys.stderr)
                return 1
            if state_path.exists():
                print("tdlib parent-death test: watchdog state остался", file=sys.stderr)
                return 1
        finally:
            if helper is not None and helper.poll() is None:
                safe_kill_group(helper.pid)
                helper.wait(timeout=5)
            safe_kill_group(target_group)
            safe_kill_group(watchdog_process)

    print("tdlib parent-death test: ok (SIGKILL parent cleanup=1)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
