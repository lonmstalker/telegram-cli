#!/usr/bin/env python3
"""Negative controls для extraction и process/resource guards native build."""

from __future__ import annotations

from io import BytesIO
import importlib.util
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import tarfile
import tempfile
import time


ROOT = Path(__file__).resolve().parent.parent
BUILDER_PATH = ROOT / "scripts/build-tdlib-native.py"


def load_builder():
    if not BUILDER_PATH.is_file():
        raise RuntimeError(f"builder отсутствует: {BUILDER_PATH.relative_to(ROOT)}")
    spec = importlib.util.spec_from_file_location("tdlib_native_builder", BUILDER_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("builder module не загружен")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def group_exists(group_id: int) -> bool:
    if group_id <= 1 or group_id == os.getpgrp():
        raise ValueError(f"unsafe process group id: {group_id}")
    try:
        os.killpg(group_id, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        result = subprocess.run(
            ["/bin/ps", "-axo", "pgid="],
            check=True,
            capture_output=True,
            text=True,
            timeout=5,
        )
        return any(line.strip() == str(group_id) for line in result.stdout.splitlines())
    return True


def assert_group_gone(group_id: int, label: str) -> None:
    if not group_exists(group_id):
        return
    raise AssertionError(f"{label}: process group {group_id} осталась запущенной")


def force_group_cleanup(group_id: int) -> None:
    """Fail-safe теста: сломанный guard не должен оставить чужой процесс."""
    if group_id <= 1 or group_id == os.getpgrp():
        return
    for action, grace in ((signal.SIGTERM, 0.2), (signal.SIGKILL, 1.0)):
        try:
            os.killpg(group_id, action)
        except ProcessLookupError:
            return
        except PermissionError:
            if not group_exists(group_id):
                return
        deadline = time.monotonic() + grace
        while time.monotonic() < deadline:
            if not group_exists(group_id):
                return
            time.sleep(0.01)


def add_tar_member(bundle: tarfile.TarFile, name: str, payload: bytes = b"x") -> None:
    member = tarfile.TarInfo(name)
    member.size = len(payload)
    bundle.addfile(member, BytesIO(payload))


def expect_extract_rejected(builder, root: Path, label: str, members) -> None:
    archive = root / f"{label}.tar.gz"
    with tarfile.open(archive, "w:gz") as bundle:
        for member, payload in members:
            if isinstance(member, str):
                add_tar_member(bundle, member, payload)
            else:
                bundle.addfile(member, BytesIO(payload) if payload else None)
    destination = root / f"extract-{label}"
    try:
        builder.safe_extract(
            archive,
            destination,
            maximum_bytes=1024,
            expected_root="tdlib-source",
            maximum_members=2,
            maximum_member_bytes=1024,
        )
    except ValueError:
        return
    raise AssertionError(f"unsafe tar принят: {label}")


def expect_guard_rejected(builder, groups: set[int], label: str, **arguments) -> None:
    try:
        builder.run_guarded(**arguments)
    except builder.BuildGuardError as error:
        if error.process_group_id <= 1 or error.process_group_id == os.getpgrp():
            raise AssertionError(
                f"{label}: guard вернул unsafe PGID {error.process_group_id}"
            ) from error
        groups.add(error.process_group_id)
        assert_group_gone(error.process_group_id, label)
        return
    raise AssertionError(f"process guard не сработал: {label}")


def main() -> int:
    try:
        builder = load_builder()
    except RuntimeError as error:
        print(f"tdlib native guard test: {error}", file=sys.stderr)
        return 1

    groups: set[int] = set()
    try:
        native = sys.modules["tdlib_native"]
        parent_socket, child_socket = socket.socketpair()
        try:
            handshake = (
                json.dumps(
                    {"event": "started", "target_process_group_id": 424242},
                    sort_keys=True,
                    separators=(",", ":"),
                ).encode("ascii")
                + b"\n"
            )
            split = len(handshake) // 2
            child_socket.sendall(handshake[:split])
            events, fragment, closed = native._read_watchdog_events(
                parent_socket, b"", 0.1
            )
            if events or closed or fragment != handshake[:split]:
                raise AssertionError("partial watchdog handshake был принят как complete")
            child_socket.sendall(handshake[split:])
            child_socket.shutdown(socket.SHUT_WR)
            events, fragment, closed = native._read_watchdog_events(
                parent_socket, fragment, 0.1
            )
            if events != [
                {"event": "started", "target_process_group_id": 424242}
            ] or fragment:
                raise AssertionError("fragmented watchdog handshake потерян")
            if not closed:
                _, fragment, closed = native._read_watchdog_events(
                    parent_socket, fragment, 0.1
                )
            if not closed or fragment:
                raise AssertionError("watchdog handshake stream не drained до EOF")
        finally:
            parent_socket.close()
            child_socket.close()

        watchdog_source = builder.read_bounded(
            ROOT / "scripts/process-group-watchdog.py",
            128 * 1024,
            "watchdog recipe",
        ).decode("utf-8")
        ordering = (
            watchdog_source.index('atomic_write_state(state_path, state("running"))'),
            watchdog_source.index("if not send_event("),
            watchdog_source.index('if os.write(gate_write, b"1") != 1:'),
        )
        if ordering != tuple(sorted(ordering)):
            print(
                "tdlib native guard test: target gate открыт до parent handshake",
                file=sys.stderr,
            )
            return 1
        with tempfile.TemporaryDirectory(prefix="tdlib-native-guard-") as directory:
            root = Path(directory)
            unsafe_members = [
                ("traversal", [("../escape", b"escape")]),
                ("absolute", [("/tdlib-source/escape", b"escape")]),
                ("second-root", [("other/file", b"escape")]),
                (
                    "symlink",
                    [
                        (
                            tarfile.TarInfo("tdlib-source/link"),
                            b"",
                        )
                    ],
                ),
                (
                    "hardlink",
                    [
                        (
                            tarfile.TarInfo("tdlib-source/hard"),
                            b"",
                        )
                    ],
                ),
                (
                    "fifo",
                    [
                        (
                            tarfile.TarInfo("tdlib-source/fifo"),
                            b"",
                        )
                    ],
                ),
                ("oversized", [("tdlib-source/large", b"x" * 1025)]),
                (
                    "members",
                    [
                        ("tdlib-source/a", b"a"),
                        ("tdlib-source/b", b"b"),
                        ("tdlib-source/c", b"c"),
                    ],
                ),
            ]
            unsafe_members[3][1][0][0].type = tarfile.SYMTYPE
            unsafe_members[3][1][0][0].linkname = "../../escape"
            unsafe_members[4][1][0][0].type = tarfile.LNKTYPE
            unsafe_members[4][1][0][0].linkname = "../../escape"
            unsafe_members[5][1][0][0].type = tarfile.FIFOTYPE
            for label, members in unsafe_members:
                expect_extract_rejected(builder, root, label, members)
            if (root / "escape").exists():
                print("tdlib native guard test: path traversal записал файл", file=sys.stderr)
                return 1

            descendant_script = (
                "import subprocess, sys; "
                "subprocess.Popen("
                "[sys.executable, '-c', 'import time; time.sleep(60)'], "
                "stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, "
                "stderr=subprocess.DEVNULL, close_fds=True)"
            )
            metrics = builder.run_guarded(
                [sys.executable, "-c", descendant_script],
                cwd=root,
                environment={"PATH": os.environ["PATH"]},
                log_path=root / "descendant.log",
                timeout_seconds=2.0,
                maximum_tree_bytes=1024 * 1024,
                maximum_group_rss_bytes=256 * 1024 * 1024,
                maximum_log_bytes=1024 * 1024,
                poll_seconds=0.02,
            )
            groups.add(metrics.process_group_id)
            assert_group_gone(metrics.process_group_id, "normal leader exit")

            base_arguments = {
                "cwd": root,
                "environment": {"PATH": os.environ["PATH"]},
                "timeout_seconds": 2.0,
                "maximum_tree_bytes": 1024 * 1024,
                "maximum_group_rss_bytes": 256 * 1024 * 1024,
                "maximum_log_bytes": 1024 * 1024,
                "poll_seconds": 0.02,
            }
            guarded_cases = (
                (
                    "disk cap",
                    "from pathlib import Path; import time; "
                    "Path('oversized').open('wb').truncate(2097152); time.sleep(60)",
                    {"maximum_tree_bytes": 1024 * 1024},
                ),
                ("timeout", "import time; time.sleep(60)", {"timeout_seconds": 0.1}),
                (
                    "RSS cap",
                    "import time; payload=bytearray(33554432); time.sleep(60)",
                    {"maximum_group_rss_bytes": 1024 * 1024},
                ),
                (
                    "log cap",
                    "import sys,time; sys.stdout.write('x'*2048); sys.stdout.flush(); time.sleep(60)",
                    {"maximum_log_bytes": 1024},
                ),
                (
                    "process count cap",
                    "import subprocess,sys,time; "
                    "subprocess.Popen([sys.executable,'-c','import time;time.sleep(60)']); "
                    "time.sleep(60)",
                    {"maximum_group_processes": 1},
                ),
                ("non-zero exit", "raise SystemExit(7)", {}),
            )
            for index, (label, script, overrides) in enumerate(guarded_cases):
                arguments = {
                    **base_arguments,
                    **overrides,
                    "command": [sys.executable, "-c", script],
                    "log_path": root / f"guard-{index}.log",
                }
                expect_guard_rejected(builder, groups, label, **arguments)

            lock_path = root / "native.lock"
            with builder.exclusive_build_lock(lock_path):
                try:
                    with builder.exclusive_build_lock(lock_path):
                        pass
                except builder.NativeBuildError:
                    pass
                else:
                    print("tdlib native guard test: второй lock принят", file=sys.stderr)
                    return 1
    finally:
        for group_id in groups:
            force_group_cleanup(group_id)

    print(
        "tdlib native guard test: ok "
        "(tar=8, process_group=7, lock=1, gate_order=1, handshake_fragments=1)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
