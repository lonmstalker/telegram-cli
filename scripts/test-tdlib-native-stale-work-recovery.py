#!/usr/bin/env python3
"""Negative controls для crash-recovery private native scratch."""

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
    if group_id <= 1 or group_id == os.getpgrp():
        raise ValueError(f"unsafe process group id: {group_id}")
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


def wait_for_path(path: Path, helper: subprocess.Popen[bytes]) -> bool:
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        if path.exists():
            return True
        if helper.poll() is not None:
            return False
        time.sleep(0.02)
    return False


def recovery_arguments(native_root: Path, build_lease) -> dict[str, object]:
    return {
        "build_lease": build_lease,
        "native_root": native_root,
        "cleanup_seconds": 10.0,
        "maximum_directories": 1,
        "maximum_guard_states": 2,
        "maximum_entries": 500_000,
    }


def assert_fail_closed_controls(builder, native_root: Path, lock_path: Path) -> None:
    sibling = native_root.parent / "sibling-sentinel"
    sibling.write_text("keep\n", encoding="utf-8")
    malformed = native_root / ".work-00000000000000000000000000000000"
    malformed.mkdir(mode=0o700)
    try:
        with builder.exclusive_build_lock(lock_path) as build_lease:
            builder.cleanup_stale_work_directories(
                **recovery_arguments(native_root, build_lease)
            )
    except builder.NativeBuildError:
        pass
    else:
        raise AssertionError("malformed owner metadata не заблокировала recovery")
    if not malformed.is_dir() or sibling.read_text(encoding="utf-8") != "keep\n":
        raise AssertionError("fail-closed recovery затронула чужой path")
    malformed.rmdir()

    symlink = native_root / ".work-11111111111111111111111111111111"
    symlink.symlink_to(sibling)
    try:
        with builder.exclusive_build_lock(lock_path) as build_lease:
            builder.cleanup_stale_work_directories(
                **recovery_arguments(native_root, build_lease)
            )
    except builder.NativeBuildError:
        pass
    else:
        raise AssertionError("symlink scratch не заблокировал recovery")
    if not symlink.is_symlink() or sibling.read_text(encoding="utf-8") != "keep\n":
        raise AssertionError("symlink recovery затронула sibling")
    symlink.unlink()

    unmarked_reap = native_root / ".reap-22222222222222222222222222222222"
    unmarked_reap.mkdir(mode=0o700)
    try:
        with builder.exclusive_build_lock(lock_path) as build_lease:
            builder.cleanup_stale_work_directories(
                **recovery_arguments(native_root, build_lease)
            )
    except builder.NativeBuildError:
        pass
    else:
        raise AssertionError("unmarked reap directory не заблокировала recovery")
    if not unmarked_reap.is_dir() or sibling.read_text(encoding="utf-8") != "keep\n":
        raise AssertionError("unmarked reap recovery затронула чужой path")
    unmarked_reap.rmdir()

    reap_symlink = native_root / ".reap-33333333333333333333333333333333"
    reap_symlink.symlink_to(sibling)
    try:
        with builder.exclusive_build_lock(lock_path) as build_lease:
            builder.cleanup_stale_work_directories(
                **recovery_arguments(native_root, build_lease)
            )
    except builder.NativeBuildError:
        pass
    else:
        raise AssertionError("reap symlink не заблокировал recovery")
    if not reap_symlink.is_symlink() or sibling.read_text(encoding="utf-8") != "keep\n":
        raise AssertionError("reap symlink recovery затронула sibling")
    reap_symlink.unlink()

    malformed_proof = (
        native_root / ".reap-proof-44444444444444444444444444444444.json"
    )
    malformed_proof.write_text("{}\n", encoding="utf-8")
    malformed_proof.chmod(0o600)
    try:
        with builder.exclusive_build_lock(lock_path) as build_lease:
            builder.cleanup_stale_work_directories(
                **recovery_arguments(native_root, build_lease)
            )
    except builder.NativeBuildError:
        pass
    else:
        raise AssertionError("malformed reap proof не заблокировал recovery")
    if not malformed_proof.is_file() or sibling.read_text(encoding="utf-8") != "keep\n":
        raise AssertionError("malformed proof recovery затронула чужой path")
    malformed_proof.unlink()

    proof_symlink = (
        native_root / ".reap-proof-55555555555555555555555555555555.json"
    )
    proof_symlink.symlink_to(sibling)
    try:
        with builder.exclusive_build_lock(lock_path) as build_lease:
            builder.cleanup_stale_work_directories(
                **recovery_arguments(native_root, build_lease)
            )
    except builder.NativeBuildError:
        pass
    else:
        raise AssertionError("reap proof symlink не заблокировал recovery")
    if not proof_symlink.is_symlink() or sibling.read_text(encoding="utf-8") != "keep\n":
        raise AssertionError("proof symlink recovery затронула sibling")
    proof_symlink.unlink()


def assert_finalization_recovery(builder, native_root: Path, lock_path: Path) -> None:
    ready_path = native_root.parent / "finalization-ready.json"
    helper_code = (
        "import json,os,signal,sys; "
        f"sys.path.insert(0,{str(ROOT / 'scripts')!r}); "
        "from pathlib import Path; import tdlib_native as native; "
        f"root=Path({str(native_root)!r}); lock=Path({str(lock_path)!r}); "
        f"ready=Path({str(ready_path)!r}); "
        "\nwith native.exclusive_build_lock(lock):"
        "\n with native.owned_work_directory(root) as work:"
        "\n  reap=root/f'{native.REAP_PREFIX}{work.work_id}'"
        "\n  proof=root/f'{native.REAP_PROOF_PREFIX}{work.work_id}{native.REAP_PROOF_SUFFIX}'"
        "\n  work.path.rename(reap)"
        "\n  (reap/native.WORK_MARKER).rename(proof)"
        "\n  ready.write_text(json.dumps({'reap':str(reap),'proof':str(proof)})+'\\n',encoding='utf-8')"
        "\n  os.kill(os.getpid(),signal.SIGKILL)"
    )
    helper = subprocess.Popen(
        [sys.executable, "-c", helper_code],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        close_fds=True,
        start_new_session=True,
    )
    try:
        if not wait_for_path(ready_path, helper):
            raise AssertionError("finalization helper не создал proof pair")
        helper.wait(timeout=5)
        paths = json.loads(ready_path.read_text(encoding="utf-8"))
        reap_path = Path(paths["reap"])
        proof_path = Path(paths["proof"])
        if not reap_path.is_dir() or not proof_path.is_file():
            raise AssertionError("finalization crash window не воспроизведён")
        with builder.exclusive_build_lock(lock_path) as build_lease:
            removed = builder.cleanup_stale_work_directories(
                **recovery_arguments(native_root, build_lease)
            )
        if removed != 1 or reap_path.exists() or proof_path.exists():
            raise AssertionError("finalization proof pair не восстановлен")
    finally:
        if helper.poll() is None:
            safe_kill_group(helper.pid)
            helper.wait(timeout=5)


def main() -> int:
    builder = load_builder()
    required = (
        "owned_work_directory",
        "cleanup_stale_work_directories",
        "exclusive_build_lock",
    )
    missing = [name for name in required if not hasattr(builder, name)]
    if missing:
        print(
            "tdlib stale-work test: production API отсутствует: " + ", ".join(missing),
            file=sys.stderr,
        )
        return 1

    helper: subprocess.Popen[bytes] | None = None
    target_group = -1
    watchdog_process = -1
    with tempfile.TemporaryDirectory(prefix="tdlib-stale-work-") as directory:
        root = Path(directory)
        native_root = root / "native"
        native_root.mkdir(mode=0o700)
        lock_path = native_root / ".build.lock"
        ready_path = root / "ready.json"
        helper_code = (
            "import json,os,sys; "
            f"sys.path.insert(0,{str(ROOT / 'scripts')!r}); "
            "from pathlib import Path; "
            "from tdlib_native import exclusive_build_lock,owned_work_directory,run_guarded; "
            f"native=Path({str(native_root)!r}); ready=Path({str(ready_path)!r}); "
            f"lock=Path({str(lock_path)!r}); "
            "\nwith exclusive_build_lock(lock) as build_lease:"
            "\n with owned_work_directory(native) as work:"
            "\n  (work.path/'sentinel').write_text('owned\\n',encoding='utf-8')"
            "\n  ready.write_text(json.dumps({'work':str(work.path),'work_id':work.work_id})+'\\n',encoding='utf-8')"
            "\n  run_guarded([sys.executable,'-c',\"from pathlib import Path;import signal,time;signal.signal(signal.SIGTERM,signal.SIG_IGN);Path('target-ready').write_text('ready\\\\n',encoding='utf-8');time.sleep(60)\"],"
            "cwd=work.path,environment={'PATH':os.environ['PATH']},"
            "log_path=work.path/'guard.log',timeout_seconds=60,"
            "maximum_tree_bytes=1048576,maximum_group_rss_bytes=268435456,"
            "maximum_log_bytes=1048576,poll_seconds=0.05,"
            "work_id=work.work_id,keepalive_fds=(build_lease.descriptor,))"
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
            if not wait_for_path(ready_path, helper):
                print("tdlib stale-work test: owner не создал scratch", file=sys.stderr)
                return 1
            ready = json.loads(ready_path.read_text(encoding="utf-8"))
            work_path = Path(ready["work"])
            state_path = builder.guard_state_path(work_path / "guard.log")
            if not wait_for_path(state_path, helper):
                print("tdlib stale-work test: watchdog state не создан", file=sys.stderr)
                return 1
            metadata = state_path.lstat()
            if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > 4096:
                print("tdlib stale-work test: watchdog state unsafe", file=sys.stderr)
                return 1
            state = json.loads(state_path.read_text(encoding="utf-8"))
            target_group = state["target_process_group_id"]
            watchdog_process = state["watchdog_process_id"]
            if state["work_id"] != ready["work_id"] or state["phase"] != "running":
                print("tdlib stale-work test: ownership state inconsistent", file=sys.stderr)
                return 1
            if not wait_for_path(work_path / "target-ready", helper):
                print("tdlib stale-work test: target не вошёл в guarded phase", file=sys.stderr)
                return 1

            os.kill(helper.pid, signal.SIGKILL)
            helper.wait(timeout=5)

            # Global lease должен не пустить второй owner, пока watchdog чистит PGID.
            try:
                with builder.exclusive_build_lock(
                    lock_path, wait_seconds=0.01
                ):
                    pass
            except builder.NativeBuildError:
                pass
            else:
                print("tdlib stale-work test: live watchdog отпустил build lock", file=sys.stderr)
                return 1
            if not work_path.is_dir() or (work_path / "sentinel").read_text(encoding="utf-8") != "owned\n":
                print("tdlib stale-work test: live scratch повреждён", file=sys.stderr)
                return 1

            with builder.exclusive_build_lock(
                lock_path, wait_seconds=10
            ) as build_lease:
                removed = builder.cleanup_stale_work_directories(
                    **recovery_arguments(native_root, build_lease)
                )
                repeated = builder.cleanup_stale_work_directories(
                    **recovery_arguments(native_root, build_lease)
                )
            if removed != 1 or repeated != 0 or work_path.exists() or work_path.is_symlink():
                print("tdlib stale-work test: stale scratch не удалён idempotently", file=sys.stderr)
                return 1
            if group_exists(target_group) or process_exists(watchdog_process) or state_path.exists():
                print("tdlib stale-work test: process/state leftovers", file=sys.stderr)
                return 1

            assert_finalization_recovery(builder, native_root, lock_path)
            assert_fail_closed_controls(builder, native_root, lock_path)
        finally:
            if helper is not None and helper.poll() is None:
                safe_kill_group(helper.pid)
                helper.wait(timeout=5)
            safe_kill_group(target_group)
            safe_kill_group(watchdog_process)

    print(
        "tdlib stale-work test: ok "
        "(SIGKILL recovery=1, finalization_recovery=1, fail_closed=6)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
