#!/usr/bin/env python3
"""Проверяет exact vendored TDLib snapshot без доступа к сети."""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path
import stat
import sys
from typing import Any


ROOT = Path(__file__).resolve().parent.parent
MANIFEST_PATH = ROOT / "vendor/tdlib/manifest.json"
MAX_MANIFEST_BYTES = 16 * 1024
MAX_CMAKE_BYTES = 128 * 1024
MAX_SCHEMA_BYTES = 2 * 1024 * 1024
MAX_LICENSE_BYTES = 16 * 1024
PAYLOAD_CAPS = {
    "cmake": MAX_CMAKE_BYTES,
    "schema": MAX_SCHEMA_BYTES,
    "license": MAX_LICENSE_BYTES,
}

EXPECTED_MANIFEST: dict[str, Any] = {
    "format_version": 1,
    "upstream": {
        "repository": "https://github.com/tdlib/td",
        "commit": "07d3a0973f5113b0827a04d54a93aaaa9e288348",
        "version": "1.8.66",
    },
    "cmake": {
        "source_path": "CMakeLists.txt",
        "vendored_path": "vendor/tdlib/CMakeLists.txt",
        "sha256": "b9020710ba0ef55d684ef1dcfa90fa19ecba43ea6673f2de4efd182254ff6e06",
        "bytes": 55_226,
        "version_declaration": "project(TDLib VERSION 1.8.66 LANGUAGES CXX C)",
    },
    "schema": {
        "source_path": "td/generate/scheme/td_api.tl",
        "vendored_path": "vendor/tdlib/td_api.tl",
        "sha256": "10a00b48d557d00c0daa231a8dad38a9d0c99de78360a1e4b0b7579b28188f31",
        "bytes": 1_138_081,
        "definitions": 2_168,
        "functions": 1_010,
        "updates": 184,
        "authorization_states": 13,
    },
    "license": {
        "spdx": "BSL-1.0",
        "source_path": "LICENSE_1_0.txt",
        "vendored_path": "vendor/tdlib/LICENSE_1_0.txt",
        "sha256": "c9bff75738922193e67fa726fa225535870d2aa1059f91452c411736284ad566",
        "bytes": 1_338,
    },
}


def flatten(value: Any, prefix: str = "") -> dict[str, Any]:
    if not isinstance(value, dict):
        return {prefix: value}
    flattened: dict[str, Any] = {}
    for key, item in value.items():
        path = f"{prefix}.{key}" if prefix else str(key)
        flattened.update(flatten(item, path))
    return flattened


def validate_manifest_contract(manifest: dict[str, Any]) -> list[str]:
    if manifest == EXPECTED_MANIFEST:
        return []
    expected = flatten(EXPECTED_MANIFEST)
    actual = flatten(manifest)
    errors: list[str] = []
    for field in sorted(set(expected) | set(actual)):
        if actual.get(field) != expected.get(field):
            errors.append(
                f"manifest.{field}: ожидалось {expected.get(field)!r}, "
                f"получено {actual.get(field)!r}"
            )
    if not errors:
        errors.append("manifest structure differs from exact pin contract")
    return errors


def read_bounded(path: Path, maximum_bytes: int, label: str) -> bytes:
    metadata = path.lstat()
    if not stat.S_ISREG(metadata.st_mode):
        raise ValueError(f"{label}: expected regular file: {path}")
    if metadata.st_size > maximum_bytes:
        raise ValueError(
            f"{label}: file exceeds hard cap {maximum_bytes}: {metadata.st_size}"
        )
    with path.open("rb") as file:
        payload = file.read(maximum_bytes + 1)
    if len(payload) > maximum_bytes:
        raise ValueError(f"{label}: bounded read exceeded hard cap {maximum_bytes}")
    if len(payload) != metadata.st_size:
        raise ValueError(f"{label}: file changed during bounded read")
    return payload


def signatures(section: str) -> list[str]:
    without_comments = "\n".join(
        line.split("//", 1)[0] for line in section.splitlines()
    )
    return [signature.strip() for signature in without_comments.split(";") if signature.strip()]


def schema_inventory(schema: bytes) -> dict[str, int]:
    text = schema.decode("utf-8")
    if text.count("---functions---") != 1:
        raise ValueError("schema должна содержать ровно один ---functions--- delimiter")

    type_section, function_section = text.split("---functions---", 1)
    type_signatures = signatures(type_section)
    function_signatures = signatures(function_section)

    def result_type(signature: str) -> str:
        return signature.rsplit("=", 1)[-1].strip()

    return {
        "definitions": len(type_signatures),
        "functions": len(function_signatures),
        "updates": sum(
            result_type(signature) == "Update" for signature in type_signatures
        ),
        "authorization_states": sum(
            result_type(signature) == "AuthorizationState"
            for signature in type_signatures
        ),
    }


def validate_payload(
    manifest: dict[str, Any], payloads: dict[str, bytes]
) -> list[str]:
    errors = validate_manifest_contract(manifest)
    for label in ("cmake", "schema", "license"):
        record = manifest.get(label, {})
        payload = payloads[label]
        if record.get("bytes") != len(payload):
            errors.append(
                f"{label}.bytes: ожидалось {record.get('bytes')}, "
                f"получено {len(payload)}"
            )
        digest = hashlib.sha256(payload).hexdigest()
        if record.get("sha256") != digest:
            errors.append(
                f"{label}.sha256: ожидалось {record.get('sha256')}, "
                f"получено {digest}"
            )

    declaration = manifest["cmake"]["version_declaration"].encode("utf-8")
    if declaration not in payloads["cmake"].splitlines():
        errors.append("cmake.version_declaration отсутствует в vendored CMakeLists.txt")

    try:
        actual_inventory = schema_inventory(payloads["schema"])
    except (UnicodeDecodeError, ValueError) as error:
        errors.append(str(error))
    else:
        for field, actual in actual_inventory.items():
            expected = manifest["schema"].get(field)
            if expected != actual:
                errors.append(
                    f"schema.{field}: ожидалось {expected}, получено {actual}"
                )
    return errors


def resolve_vendored_path(record: dict[str, Any], label: str) -> Path:
    path = (ROOT / str(record["vendored_path"])).resolve()
    if path == ROOT or ROOT not in path.parents:
        raise ValueError(f"{label}.vendored_path выходит за границы repository")
    return path


def negative_control_errors(
    manifest: dict[str, Any], payloads: dict[str, bytes]
) -> list[str]:
    errors: list[str] = []
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
        if not validate_manifest_contract(candidate):
            errors.append(f"negative control: {label} mutation не обнаружена")

    wrong_count = copy.deepcopy(manifest)
    wrong_count["schema"]["functions"] += 1
    if not any(
        error.startswith("schema.functions:")
        for error in validate_payload(wrong_count, payloads)
    ):
        errors.append("negative control: schema function count drift не обнаружен")

    corrupted_payloads = dict(payloads)
    corrupted_payloads["schema"] = payloads["schema"] + b"\n"
    if not any(
        error.startswith("schema.sha256:")
        for error in validate_payload(manifest, corrupted_payloads)
    ):
        errors.append("negative control: schema content drift не обнаружен")
    return errors


def main() -> int:
    if not MANIFEST_PATH.is_file():
        print(f"tdlib pin: отсутствует {MANIFEST_PATH.relative_to(ROOT)}", file=sys.stderr)
        return 1

    try:
        manifest = json.loads(
            read_bounded(MANIFEST_PATH, MAX_MANIFEST_BYTES, "manifest").decode("utf-8")
        )
    except (OSError, UnicodeDecodeError, ValueError, json.JSONDecodeError) as error:
        print(f"tdlib pin: {error}", file=sys.stderr)
        return 1

    errors = validate_manifest_contract(manifest)
    if errors:
        for error in errors:
            print(f"tdlib pin: {error}", file=sys.stderr)
        return 1

    try:
        payloads = {
            label: read_bounded(
                resolve_vendored_path(manifest[label], label),
                PAYLOAD_CAPS[label],
                label,
            )
            for label in ("cmake", "schema", "license")
        }
    except (KeyError, OSError, ValueError) as error:
        print(f"tdlib pin: {error}", file=sys.stderr)
        return 1

    errors = validate_payload(manifest, payloads)
    errors.extend(negative_control_errors(manifest, payloads))
    if errors:
        for error in errors:
            print(f"tdlib pin: {error}", file=sys.stderr)
        return 1

    inventory = schema_inventory(payloads["schema"])
    print(
        "tdlib pin: ok "
        f"(functions={inventory['functions']}, definitions={inventory['definitions']}, "
        f"updates={inventory['updates']}, auth_states={inventory['authorization_states']}, "
        "negative_controls=7)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
