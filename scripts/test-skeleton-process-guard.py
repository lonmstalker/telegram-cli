#!/usr/bin/env python3
"""Negative control для timeout и cleanup process group skeleton-checker."""

from __future__ import annotations

import importlib.util
import os
from pathlib import Path
import signal
import sys


ROOT = Path(__file__).resolve().parent.parent
CHECKER = ROOT / "scripts/check-skeleton-fails-closed.py"


def main() -> int:
    spec = importlib.util.spec_from_file_location("skeleton_contract", CHECKER)
    if spec is None or spec.loader is None:
        print("process guard test: checker module не загружен", file=sys.stderr)
        return 1

    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    run_bounded = getattr(module, "run_bounded", None)
    if run_bounded is None:
        print("process guard test: run_bounded отсутствует", file=sys.stderr)
        return 1

    timed_out_result = run_bounded(
        [sys.executable, "-c", "import time; time.sleep(60)"],
        environment=os.environ.copy(),
        timeout_seconds=0.05,
        terminate_grace_seconds=0.05,
    )
    if not timed_out_result.timed_out:
        print("process guard test: timeout не сработал", file=sys.stderr)
        return 1

    try:
        os.killpg(timed_out_result.pid, 0)
    except ProcessLookupError:
        pass
    else:
        os.killpg(timed_out_result.pid, signal.SIGKILL)
        print("process guard test: timed-out group осталась запущенной", file=sys.stderr)
        return 1

    leader_script = (
        "import subprocess, sys; "
        "subprocess.Popen("
        "[sys.executable, '-c', 'import time; time.sleep(60)'], "
        "stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, "
        "stderr=subprocess.DEVNULL, close_fds=True)"
    )
    descendant_result = run_bounded(
        [sys.executable, "-c", leader_script],
        environment=os.environ.copy(),
        timeout_seconds=1.0,
        terminate_grace_seconds=0.05,
    )
    try:
        os.killpg(descendant_result.pid, 0)
    except ProcessLookupError:
        print("process guard test: ok")
        return 0
    else:
        os.killpg(descendant_result.pid, signal.SIGKILL)

    print(
        "process guard test: descendant остался после normal leader exit",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
