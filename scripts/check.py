#!/usr/bin/env python3
"""Small serial, offline Rust/CLI verification harness. No native rebuilds or Telegram calls."""
import argparse
import fcntl
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["fast", "verify", "build", "release"], default="fast", nargs="?")
    mode = parser.parse_args().mode
    target = ROOT / "target"; target.mkdir(exist_ok=True)
    with (target / ".check.lock").open("a") as lock:
        try: fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError: raise SystemExit("another harness run is active")
        commands = []
        if mode in ("fast", "verify"):
            commands = [[sys.executable, "scripts/check-workspace-boundaries.py"],
                        [sys.executable, "scripts/check-tdlib-pin.py"],
                        ["cargo", "fmt", "--all", "--", "--check"],
                        ["cargo", "check", "--locked", "--offline", "-q"]]
        if mode == "verify":
            commands += [["cargo", "clippy", "--locked", "--offline", "-q", "--all-targets", "--", "-D", "warnings"],
                         ["cargo", "test", "--locked", "--offline", "-q", "--", "--test-threads=2"],
                         ["cargo", "build", "--locked", "--offline", "-q", "-p", "telegram-cli", "-p", "telegramd"],
                         [sys.executable, "scripts/test-agent-cli.py"],
                         [sys.executable, "scripts/test-rotate-wiki-journal.py"],
                         ["sh", "-n", "install.sh"], ["sh", "-n", "scripts/package-release.sh"],
                         [sys.executable, "scripts/rotate-wiki-journal.py", "--all", "--check"]]
        if mode in ("build", "release"):
            commands = [["cargo", "build", "--locked", "--offline", "-q", "-p", "telegram-cli", "-p", "telegramd"] + (["--release"] if mode == "release" else [])]
        for command in commands:
            result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
            if result.returncode:
                print(result.stdout + result.stderr, end="", file=sys.stderr)
                raise SystemExit(result.returncode)
        print(f"{mode}: {len(commands)} checks passed")


if __name__ == "__main__":
    main()
