#!/usr/bin/env python3
"""Проверяет один Linux x86_64 TDJSON artifact внутри pinned builder image."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile


REQUIRED_EXPORTS = (
    "td_json_client_create",
    "td_json_client_send",
    "td_json_client_receive",
    "td_json_client_execute",
    "td_json_client_destroy",
)


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--artifact", required=True, type=Path)
    parser.add_argument("--expected-version", required=True)
    parser.add_argument("--expected-commit", required=True)
    parser.add_argument("--expected-soname", required=True)
    parser.add_argument("--smoke-script", required=True, type=Path)
    return parser.parse_args()


def capture(command: list[str]) -> str:
    result = subprocess.run(
        command,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=20,
    )
    if result.returncode != 0:
        detail = result.stderr.strip().splitlines()[-1:] or ["no stderr"]
        raise RuntimeError(f"command failed: {command[0]}: {detail[0]}")
    if len(result.stdout.encode("utf-8")) > 2 * 1024 * 1024:
        raise RuntimeError(f"command output exceeds cap: {command[0]}")
    return result.stdout.strip()


def header_field(header: str, name: str) -> str:
    match = re.search(rf"^\s*{re.escape(name)}:\s*(.+)$", header, re.MULTILINE)
    if match is None:
        raise RuntimeError(f"ELF header misses {name}")
    return match.group(1).strip()


def main() -> int:
    arguments = parse_arguments()
    artifact = arguments.artifact.resolve(strict=True)
    if not artifact.is_file() or artifact.is_symlink():
        raise RuntimeError("artifact must be a regular file without symlink")

    file_format = capture(["file", "-b", str(artifact)])
    header = capture(["readelf", "-h", str(artifact)])
    elf = {
        "class": header_field(header, "Class"),
        "data": header_field(header, "Data"),
        "type": header_field(header, "Type").split(maxsplit=1)[0],
        "machine": header_field(header, "Machine"),
    }
    expected_elf = {
        "class": "ELF64",
        "data": "2's complement, little endian",
        "type": "DYN",
        "machine": "Advanced Micro Devices X86-64",
    }
    if elf != expected_elf:
        raise RuntimeError(f"artifact ELF identity differs: {elf}")

    dynamic = capture(["readelf", "-d", str(artifact)])
    if "(RPATH)" in dynamic or "(RUNPATH)" in dynamic:
        raise RuntimeError("artifact contains RPATH/RUNPATH")
    dependencies = re.findall(r"\(NEEDED\).*Shared library: \[([^]]+)\]", dynamic)
    sonames = re.findall(r"\(SONAME\).*Library soname: \[([^]]+)\]", dynamic)
    if sonames != [arguments.expected_soname]:
        raise RuntimeError(f"artifact SONAME differs: {sonames}")
    if not dependencies or len(dependencies) != len(set(dependencies)):
        raise RuntimeError("artifact dynamic dependencies are empty or duplicated")

    symbols = capture(["nm", "-D", "--defined-only", "--format=posix", str(artifact)])
    exported = {line.split(maxsplit=1)[0] for line in symbols.splitlines() if line}
    missing = sorted(set(REQUIRED_EXPORTS) - exported)
    if missing:
        raise RuntimeError("artifact misses TDJSON exports: " + ", ".join(missing))

    with tempfile.TemporaryDirectory(prefix="tdjson-smoke-") as directory:
        output = Path(directory) / "result.json"
        capture(
            [
                sys.executable,
                str(arguments.smoke_script),
                "--artifact",
                str(artifact),
                "--expected-version",
                arguments.expected_version,
                "--expected-commit",
                arguments.expected_commit,
                "--output",
                str(output),
            ]
        )
        smoke = json.loads(output.read_text(encoding="utf-8"))

    print(
        json.dumps(
            {
                "file_format": file_format,
                "elf": elf,
                "soname": sonames[0],
                "rpaths": [],
                "dynamic_dependencies": dependencies,
                "exports": list(REQUIRED_EXPORTS),
                "options": smoke["options"],
                "database_files_created": smoke["database_files_created"],
                "glibc": capture(["ldd", "--version"]).splitlines()[0],
            },
            sort_keys=True,
            separators=(",", ":"),
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.SubprocessError, ValueError) as error:
        print(f"tdlib linux inspect: {error}", file=sys.stderr)
        raise SystemExit(1) from error
