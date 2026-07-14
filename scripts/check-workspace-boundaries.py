#!/usr/bin/env python3
"""Проверяет физические границы Cargo workspace через `cargo metadata`."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parent.parent

EXPECTED = {
    "telegram-protocol": ("lib", set()),
    "telegram-core": ("lib", set()),
    "telegramd": ("bin", {"telegram-core", "telegram-protocol"}),
    "telegram-cli": ("bin", {"telegram-protocol"}),
    "telegram-mcp": ("bin", {"telegram-protocol"}),
    "telegram-webapp-runner": ("bin", {"telegram-protocol"}),
}
DEFAULT_MEMBERS = set(EXPECTED) - {"telegram-mcp"}
EXPECTED_MANIFESTS = {
    "telegram-protocol": ROOT / "crates/telegram-protocol/Cargo.toml",
    "telegram-core": ROOT / "crates/telegram-core/Cargo.toml",
    "telegramd": ROOT / "apps/telegramd/Cargo.toml",
    "telegram-cli": ROOT / "apps/telegram-cli/Cargo.toml",
    "telegram-mcp": ROOT / "apps/telegram-mcp/Cargo.toml",
    "telegram-webapp-runner": ROOT / "apps/telegram-webapp-runner/Cargo.toml",
}


def fail(message: str) -> None:
    print(f"workspace contract: {message}", file=sys.stderr)


def validate(metadata: dict[str, object]) -> list[str]:
    member_ids = set(metadata["workspace_members"])
    packages = {
        package["name"]: package
        for package in metadata["packages"]
        if package["id"] in member_ids
    }

    errors: list[str] = []
    expected_names = set(EXPECTED)
    actual_names = set(packages)
    if actual_names != expected_names:
        errors.append(
            "workspace members: "
            f"ожидались {sorted(expected_names)}, получены {sorted(actual_names)}"
        )

    default_member_names = {
        package["name"]
        for package in metadata["packages"]
        if package["id"] in set(metadata["workspace_default_members"])
    }
    if default_member_names != DEFAULT_MEMBERS:
        errors.append(
            "default members не должны включать deferred MCP: "
            f"ожидались {sorted(DEFAULT_MEMBERS)}, "
            f"получены {sorted(default_member_names)}"
        )

    for package_name, (expected_kind, expected_local_dependencies) in EXPECTED.items():
        package = packages.get(package_name)
        if package is None:
            continue

        target_kinds = {kind for target in package["targets"] for kind in target["kind"]}
        if target_kinds != {expected_kind}:
            errors.append(
                f"{package_name}: ожидался единственный Cargo target kind "
                f"`{expected_kind}`, получены {sorted(target_kinds)}"
            )

        manifest_path = Path(package["manifest_path"]).resolve()
        expected_manifest_path = EXPECTED_MANIFESTS[package_name].resolve()
        if manifest_path != expected_manifest_path:
            errors.append(
                f"{package_name}: ожидался manifest {expected_manifest_path}, "
                f"получен {manifest_path}"
            )

        local_dependencies = {
            dependency["name"]
            for dependency in package["dependencies"]
            if dependency.get("path") is not None
        }
        if local_dependencies != expected_local_dependencies:
            errors.append(
                f"{package_name}: ожидались локальные зависимости "
                f"{sorted(expected_local_dependencies)}, получены "
                f"{sorted(local_dependencies)}"
            )

    return errors


def validate_negative_controls(metadata: dict[str, object]) -> list[str]:
    errors: list[str] = []

    extra_target = copy.deepcopy(metadata)
    telegramd = next(
        package
        for package in extra_target["packages"]
        if package["name"] == "telegramd"
    )
    telegramd["targets"][0]["kind"].append("lib")
    if not validate(extra_target):
        errors.append("negative control: лишний target kind не обнаружен")

    wrong_manifest = copy.deepcopy(metadata)
    protocol = next(
        package
        for package in wrong_manifest["packages"]
        if package["name"] == "telegram-protocol"
    )
    protocol["manifest_path"] = str(ROOT / "unexpected/Cargo.toml")
    if not validate(wrong_manifest):
        errors.append("negative control: неверный manifest path не обнаружен")

    return errors


def main() -> int:
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        fail("`cargo metadata` завершился с ошибкой")
        if result.stderr:
            print(result.stderr.rstrip(), file=sys.stderr)
        return 1

    metadata = json.loads(result.stdout)
    errors = validate(metadata)
    errors.extend(validate_negative_controls(metadata))
    if errors:
        for error in errors:
            fail(error)
        return 1

    print("workspace contract: ok (negative controls: 2)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
