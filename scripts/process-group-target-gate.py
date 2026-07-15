#!/usr/bin/env python3
"""Не запускает guarded target до durable watchdog reservation."""

from __future__ import annotations

import argparse
import os
import sys


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--gate-fd", type=int, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    arguments = parser.parse_args()
    if arguments.command[:1] == ["--"]:
        arguments.command = arguments.command[1:]
    if not arguments.command:
        parser.error("target command is required")
    return arguments


def main() -> int:
    arguments = parse_arguments()
    if arguments.gate_fd <= 2:
        raise ValueError("unsafe target gate descriptor")
    try:
        permission = os.read(arguments.gate_fd, 1)
    finally:
        os.close(arguments.gate_fd)
    if permission != b"1":
        return 75
    os.execvpe(arguments.command[0], arguments.command, dict(os.environ))
    raise RuntimeError("target exec unexpectedly returned")


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, RuntimeError) as error:
        print(f"process-group target gate: {error}", file=sys.stderr)
        raise SystemExit(1) from error
