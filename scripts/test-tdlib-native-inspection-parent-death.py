#!/usr/bin/env python3
"""Доказывает global lease и recovery при SIGKILL внутри inspect_artifact."""

from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import signal
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


def wait_for(predicate, helper: subprocess.Popen[bytes]) -> bool:
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        if predicate():
            return True
        if helper.poll() is not None:
            return False
        time.sleep(0.02)
    return False


def main() -> int:
    builder = load_builder()
    helper: subprocess.Popen[bytes] | None = None
    target_group = -1
    watchdog_process = -1
    with tempfile.TemporaryDirectory(prefix="tdlib-inspection-death-") as directory:
        root = Path(directory)
        native_root = root / "native"
        native_root.mkdir(mode=0o700)
        lock_path = native_root / ".build.lock"
        artifact = root / "dummy-artifact"
        artifact.write_bytes(b"not-a-real-dylib\n")
        ready_path = root / "inspection-ready.json"
        helper_code = (
            "import json,os,sys; "
            f"sys.path.insert(0,{str(ROOT / 'scripts')!r}); "
            "from pathlib import Path; import tdlib_native as native; "
            f"root=Path({str(native_root)!r}); lock=Path({str(lock_path)!r}); "
            f"ready=Path({str(ready_path)!r}); artifact=Path({str(artifact)!r}); "
            "policy,_=native.load_exact_contracts(); original=native.capture_command; "
            "\nwith native.exclusive_build_lock(lock) as lease:"
            "\n with native.owned_work_directory(root) as work:"
            "\n  def delayed(command,**kwargs):"
            "\n   if kwargs.get('scratch_root')!=work.path or kwargs.get('work_id')!=work.work_id or kwargs.get('keepalive_fds')!=(lease.descriptor,): raise RuntimeError('inspection lease was not threaded')"
            "\n   ready.write_text(json.dumps({'work':str(work.path),'work_id':work.work_id})+'\\n',encoding='utf-8')"
            "\n   script=\"from pathlib import Path;import signal,time;signal.signal(signal.SIGTERM,signal.SIG_IGN);Path('inspection-target-ready').write_text('ready\\\\n',encoding='utf-8');time.sleep(60)\""
            "\n   return original([sys.executable,'-c',script],timeout_seconds=60,maximum_rss_bytes=268435456,scratch_root=work.path,work_id=work.work_id,keepalive_fds=(lease.descriptor,))"
            "\n  native.capture_command=delayed"
            "\n  native.inspect_artifact(artifact,policy,scratch_root=work.path,work_id=work.work_id,keepalive_fds=(lease.descriptor,))"
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
            if not wait_for(ready_path.exists, helper):
                print("tdlib inspection-death test: inspection не стартовала", file=sys.stderr)
                return 1
            ready = json.loads(ready_path.read_text(encoding="utf-8"))
            work_path = Path(ready["work"])

            def inspection_ready() -> bool:
                return any(work_path.glob(".inspect-*/inspection-target-ready"))

            if not wait_for(inspection_ready, helper):
                print("tdlib inspection-death test: guarded target не готов", file=sys.stderr)
                return 1
            states = list(work_path.glob(".inspect-*/.*.guard-state.json"))
            if len(states) != 1:
                print("tdlib inspection-death test: state не найден", file=sys.stderr)
                return 1
            state = json.loads(states[0].read_text(encoding="utf-8"))
            target_group = state["target_process_group_id"]
            watchdog_process = state["watchdog_process_id"]

            os.kill(helper.pid, signal.SIGKILL)
            helper.wait(timeout=5)
            try:
                with builder.exclusive_build_lock(lock_path, wait_seconds=0.01):
                    pass
            except builder.NativeBuildError:
                pass
            else:
                print("tdlib inspection-death test: lease отпущен до cleanup", file=sys.stderr)
                return 1
            if not work_path.is_dir():
                print("tdlib inspection-death test: live scratch удалён", file=sys.stderr)
                return 1

            with builder.exclusive_build_lock(
                lock_path, wait_seconds=10
            ) as build_lease:
                removed = builder.cleanup_stale_work_directories(
                    build_lease=build_lease,
                    native_root=native_root,
                    cleanup_seconds=10,
                    maximum_directories=1,
                    maximum_guard_states=2,
                    maximum_entries=500_000,
                )
            if removed != 1 or work_path.exists() or work_path.is_symlink():
                print("tdlib inspection-death test: stale scratch не удалён", file=sys.stderr)
                return 1
            if group_exists(target_group) or process_exists(watchdog_process):
                print("tdlib inspection-death test: process leftovers", file=sys.stderr)
                return 1
        finally:
            if helper is not None and helper.poll() is None:
                safe_kill_group(helper.pid)
                helper.wait(timeout=5)
            safe_kill_group(target_group)
            safe_kill_group(watchdog_process)

    print("tdlib inspection-death test: ok (SIGKILL inspection cleanup=1)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
