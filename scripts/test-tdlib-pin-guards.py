#!/usr/bin/env python3
"""Negative controls для exact provenance, memory caps и bundle preparation."""

from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import sys


ROOT = Path(__file__).resolve().parent.parent


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"module не загружен: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main() -> int:
    checker = load_module("tdlib_pin_checker", ROOT / "scripts/check-tdlib-pin.py")
    vendor = load_module("tdlib_pin_vendor", ROOT / "scripts/vendor-tdlib-schema.py")
    required = {
        "checker.validate_manifest_contract": getattr(
            checker, "validate_manifest_contract", None
        ),
        "checker.read_bounded": getattr(checker, "read_bounded", None),
        "vendor.fetch_verified_payloads": getattr(
            vendor, "fetch_verified_payloads", None
        ),
    }
    missing = [name for name, value in required.items() if value is None]
    if missing:
        print(f"tdlib guard test: отсутствуют {', '.join(missing)}", file=sys.stderr)
        return 1

    manifest = json.loads(
        (ROOT / "vendor/tdlib/manifest.json").read_text(encoding="utf-8")
    )
    if checker.validate_manifest_contract(manifest):
        print("tdlib guard test: baseline manifest rejected", file=sys.stderr)
        return 1

    mutations = (
        ("commit", ("upstream", "commit"), "0" * 40),
        ("version", ("upstream", "version"), "9.9.9"),
        ("cmake hash", ("cmake", "sha256"), "0" * 64),
        ("schema source", ("schema", "source_path"), "other.tl"),
        ("license spdx", ("license", "spdx"), "MIT"),
    )
    for label, path, value in mutations:
        candidate = copy.deepcopy(manifest)
        candidate[path[0]][path[1]] = value
        if not checker.validate_manifest_contract(candidate):
            print(
                f"tdlib guard test: provenance mutation не обнаружена: {label}",
                file=sys.stderr,
            )
            return 1

        download_calls = 0

        def forbidden_download(
            url: str, expected_bytes: int, maximum_bytes: int
        ) -> bytes:
            nonlocal download_calls
            del url, expected_bytes, maximum_bytes
            download_calls += 1
            raise RuntimeError("exact manifest guard должен сработать до network")

        try:
            vendor.fetch_verified_payloads(candidate, downloader=forbidden_download)
        except (RuntimeError, ValueError):
            pass
        if download_calls != 0:
            print(
                f"tdlib guard test: vendor принял provenance mutation: {label}",
                file=sys.stderr,
            )
            return 1

    with tempfile.TemporaryDirectory(prefix="tdlib-pin-guard-") as directory:
        oversized = Path(directory) / "oversized"
        with oversized.open("wb") as file:
            file.truncate(checker.MAX_SCHEMA_BYTES + 1)
        try:
            checker.read_bounded(oversized, checker.MAX_SCHEMA_BYTES, "schema")
        except ValueError:
            pass
        else:
            print("tdlib guard test: oversized sparse file прочитан", file=sys.stderr)
            return 1

    local_payloads = {
        record["source_path"]: checker.read_bounded(
            ROOT / record["vendored_path"],
            checker.PAYLOAD_CAPS[label],
            label,
        )
        for label, record in (
            ("cmake", manifest["cmake"]),
            ("schema", manifest["schema"]),
            ("license", manifest["license"]),
        )
    }

    def local_download(url: str, expected_bytes: int, maximum_bytes: int) -> bytes:
        del expected_bytes, maximum_bytes
        source_path = url.split(manifest["upstream"]["commit"] + "/", 1)[1]
        return local_payloads[source_path]

    bundle = vendor.fetch_verified_payloads(manifest, downloader=local_download)
    if set(bundle) != {"cmake", "schema", "license"}:
        print("tdlib guard test: bundle подготовлен не полностью", file=sys.stderr)
        return 1

    print("tdlib guard test: ok (provenance=10, bounds=1, bundle=1)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
