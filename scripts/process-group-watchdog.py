#!/usr/bin/env python3
"""Crash-safe launcher: убирает target process group при EOF guard-parent."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import select
import signal
import socket
import stat
import subprocess
import sys
import tempfile
import time
from typing import Any


class WatchdogError(RuntimeError):
    pass


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--control-fd", type=int, required=True)
    parser.add_argument("--state-path", type=Path, required=True)
    parser.add_argument("--guard-parent-pid", type=int, required=True)
    parser.add_argument("--work-id", required=True)
    parser.add_argument("--grace-seconds", type=float, required=True)
    parser.add_argument("--cwd", type=Path, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    arguments = parser.parse_args()
    if arguments.command[:1] == ["--"]:
        arguments.command = arguments.command[1:]
    if not arguments.command:
        parser.error("target command is required")
    return arguments


def atomic_write_state(path: Path, value: dict[str, Any]) -> None:
    payload = (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode(
        "ascii"
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary: Path | None = None
    try:
        descriptor, name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
        temporary = Path(name)
        with os.fdopen(descriptor, "wb") as output:
            output.write(payload)
            output.flush()
            os.fsync(output.fileno())
        temporary.chmod(0o600)
        os.replace(temporary, path)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def send_event(control: socket.socket, event: dict[str, Any]) -> bool:
    payload = (json.dumps(event, sort_keys=True, separators=(",", ":")) + "\n").encode(
        "ascii"
    )
    try:
        control.sendall(payload)
    except (BrokenPipeError, ConnectionResetError, OSError):
        return False
    return True


def group_processes(process_group_id: int) -> int:
    result = subprocess.run(
        ["/bin/ps", "-axo", "pgid="],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=5,
        env={"PATH": "/usr/bin:/bin", "LC_ALL": "C"},
    )
    if result.returncode != 0 or len(result.stdout) > 4 * 1024 * 1024:
        raise WatchdogError("bounded process-group inspection failed")
    return sum(
        line.strip() == str(process_group_id).encode("ascii")
        for line in result.stdout.splitlines()
    )


def group_exists(process_group_id: int, target: subprocess.Popen[bytes]) -> bool:
    target.poll()
    try:
        os.killpg(process_group_id, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return group_processes(process_group_id) > 0
    return True


def cleanup_group(
    target: subprocess.Popen[bytes], process_group_id: int, grace_seconds: float
) -> None:
    if process_group_id <= 1 or process_group_id == os.getpgrp():
        raise WatchdogError(f"unsafe target process group: {process_group_id}")
    target.poll()
    if group_exists(process_group_id, target):
        try:
            os.killpg(process_group_id, signal.SIGTERM)
        except (ProcessLookupError, PermissionError):
            pass
    deadline = time.monotonic() + grace_seconds
    while time.monotonic() < deadline:
        if not group_exists(process_group_id, target):
            break
        time.sleep(0.05)
    if group_exists(process_group_id, target):
        try:
            os.killpg(process_group_id, signal.SIGKILL)
        except (ProcessLookupError, PermissionError):
            pass
    kill_deadline = time.monotonic() + max(1.0, grace_seconds)
    while time.monotonic() < kill_deadline:
        if not group_exists(process_group_id, target):
            break
        time.sleep(0.05)
    try:
        target.wait(timeout=max(1.0, grace_seconds))
    except subprocess.TimeoutExpired:
        target.kill()
        target.wait(timeout=max(1.0, grace_seconds))
    if group_exists(process_group_id, target):
        raise WatchdogError(f"target process group remains alive: {process_group_id}")


def main() -> int:
    arguments = parse_arguments()
    if (
        arguments.guard_parent_pid <= 1
        or arguments.grace_seconds <= 0
        or len(arguments.work_id) != 32
        or any(character not in "0123456789abcdef" for character in arguments.work_id)
    ):
        raise WatchdogError("invalid watchdog ownership contract")
    cwd = arguments.cwd.resolve(strict=True)
    state_path = arguments.state_path.resolve()
    if cwd != state_path.parent and cwd not in state_path.parents:
        raise WatchdogError("watchdog state path escapes guarded cwd")
    if state_path.exists() or state_path.is_symlink():
        raise WatchdogError("watchdog state path already exists")

    control = socket.socket(fileno=arguments.control_fd)
    target: subprocess.Popen[bytes] | None = None
    target_group = -1
    state_written = False
    gate_read = -1
    gate_write = -1

    def state(phase: str) -> dict[str, Any]:
        return {
            "format_version": 2,
            "work_id": arguments.work_id,
            "phase": phase,
            "guard_parent_process_id": arguments.guard_parent_pid,
            "watchdog_process_id": os.getpid(),
            "target_process_id": target.pid if target is not None else None,
            "target_process_group_id": target_group if target_group > 1 else None,
        }

    def interrupted(signum: int, _frame: Any) -> None:
        raise WatchdogError(f"watchdog interrupted by signal {signum}")

    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    try:
        atomic_write_state(state_path, state("starting"))
        state_written = True
        gate_read, gate_write = os.pipe()
        target = subprocess.Popen(
            [
                sys.executable,
                str(Path(__file__).with_name("process-group-target-gate.py")),
                "--gate-fd",
                str(gate_read),
                "--",
                *arguments.command,
            ],
            cwd=cwd,
            env=dict(os.environ),
            stdin=subprocess.DEVNULL,
            stdout=1,
            stderr=subprocess.STDOUT,
            close_fds=True,
            start_new_session=True,
            shell=False,
            pass_fds=(gate_read,),
        )
        os.close(gate_read)
        gate_read = -1
        target_group = os.getpgid(target.pid)
        atomic_write_state(state_path, state("running"))
        if not send_event(
            control,
            {
                "event": "started",
                "target_process_id": target.pid,
                "target_process_group_id": target_group,
            },
        ):
            return 0
        if os.write(gate_write, b"1") != 1:
            raise WatchdogError("target gate permission write was incomplete")
        os.close(gate_write)
        gate_write = -1

        control.setblocking(False)
        buffer = b""
        exit_reported = False
        while True:
            return_code = target.poll()
            if return_code is not None and not exit_reported:
                if not send_event(
                    control, {"event": "exited", "return_code": return_code}
                ):
                    break
                exit_reported = True
            readable, _, _ = select.select([control], [], [], 0.05)
            if not readable:
                continue
            chunk = control.recv(4096)
            if not chunk:
                break
            buffer += chunk
            if len(buffer) > 4096:
                raise WatchdogError("watchdog control message exceeded cap")
            while b"\n" in buffer:
                line, buffer = buffer.split(b"\n", 1)
                if line == b"CLEANUP":
                    return 0
                raise WatchdogError("unknown watchdog control message")
    finally:
        cleanup_error: BaseException | None = None
        for descriptor in (gate_read, gate_write):
            if descriptor >= 0:
                try:
                    os.close(descriptor)
                except OSError:
                    pass
        if target is not None and target_group > 1:
            try:
                atomic_write_state(state_path, state("cleaning"))
            except BaseException as error:
                cleanup_error = error
            try:
                cleanup_group(target, target_group, arguments.grace_seconds)
            except BaseException as error:
                cleanup_error = cleanup_error or error
        if state_written and cleanup_error is None:
            state_path.unlink(missing_ok=True)
        send_event(
            control,
            {
                "event": "cleaned",
                "target_process_group_id": target_group,
                "cleanup_ok": cleanup_error is None,
            },
        )
        control.close()
        if cleanup_error is not None:
            raise WatchdogError(f"watchdog cleanup failed: {cleanup_error}") from cleanup_error
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, WatchdogError) as error:
        print(f"process-group watchdog: {error}", file=sys.stderr)
        raise SystemExit(1) from error
