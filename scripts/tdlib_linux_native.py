#!/usr/bin/env python3
"""Exact Linux x86_64 policy, provenance и isolated ELF inspection."""

from __future__ import annotations

from datetime import datetime
import json
import os
from pathlib import Path
import platform
import secrets
import subprocess
from typing import Any, Sequence

from tdlib_native import (
    MAX_POLICY_BYTES,
    MAX_PROVENANCE_BYTES,
    NativeBuildError,
    POLICY_PATH,
    ROOT,
    artifact_cache_path,
    canonical_sha256,
    exact_contract_errors,
    read_json_bounded,
    sha256_file,
)


LINUX_POLICY_PATH = ROOT / "vendor/tdlib/native-linux-build-policy.json"
LINUX_PROVENANCE_PATH = (
    ROOT / "vendor/tdlib/native-builds/x86_64-unknown-linux-gnu.json"
)
LINUX_DOCKERFILE_PATH = (
    ROOT / "vendor/tdlib/native-builds/x86_64-unknown-linux-gnu.Dockerfile"
)
LINUX_RECIPE_PATHS = (
    "scripts/tdlib_native.py",
    "scripts/tdlib_linux_native.py",
    "scripts/build-tdlib-linux-native.py",
    "scripts/inspect-tdlib-linux-native.py",
    "scripts/smoke-tdlib-native.py",
    "vendor/tdlib/native-builds/x86_64-unknown-linux-gnu.Dockerfile",
)
LINUX_REQUIRED_EXPORTS = (
    "td_json_client_create",
    "td_json_client_send",
    "td_json_client_receive",
    "td_json_client_execute",
    "td_json_client_destroy",
)

EXPECTED_LINUX_POLICY: dict[str, Any] = {
    "format_version": 1,
    "source_contract_sha256": (
        "c680308c26290de3033250f522b301b98aabff6a37976416409587bcf0690541"
    ),
    "builder": {
        "platform": "linux/amd64",
        "base_image": (
            "debian:12-slim@sha256:"
            "63a496b5d3b99214b39f5ed70eb71a61e590a77979c79cbee4faf991f8c0783e"
        ),
        "image_tag": "telegram-cli-tdlib-linux-builder:9cee1f42a109",
        "image_id": (
            "sha256:9cee1f42a1090e063c0cbf3328ddd291257dbd956af7b4786d926ce4d94ce080"
        ),
        "packages": {
            "binutils": "2.40-2",
            "ca-certificates": "20230311+deb12u1",
            "cmake": "3.25.1-1",
            "file": "1:5.44-3",
            "g++": "4:12.2.0-3",
            "git": "1:2.39.5-0+deb12u3",
            "gperf": "3.1-1",
            "libssl-dev:amd64": "3.0.20-1~deb12u2",
            "make": "4.3-4.1",
            "pkg-config:amd64": "1.8.1-1",
            "python3": "3.11.2-1+b1",
            "zlib1g-dev:amd64": "1:1.2.13.dfsg-1",
        },
    },
    "limits": {
        "cpus": 4,
        "memory_bytes": 6 * 1024**3,
        "pids": 128,
        "configure_seconds": 900,
        "build_seconds": 5_400,
        "artifact_bytes": 64 * 1024**2,
        "build_tree_bytes": 4 * 1024**3,
        "source_archive_bytes": 8 * 1024**2,
        "source_archive_members": 50_000,
        "source_archive_member_bytes": 16 * 1024**2,
        "source_archive_path_bytes": 512,
        "source_archive_path_depth": 32,
        "extracted_source_bytes": 256 * 1024**2,
    },
    "target": {
        "triple": "x86_64-unknown-linux-gnu",
        "cmake_generator": "Unix Makefiles",
        "cmake_target": "tdjson",
        "artifact_basename": "libtdjson.so.1.8.66",
        "artifact_cache_directory": (
            "target/tdlib-native/x86_64-unknown-linux-gnu/by-sha256"
        ),
        "cmake_defines": [
            "BUILD_TESTING=OFF",
            "CCACHE_FOUND=CCACHE_FOUND-NOTFOUND",
            "CMAKE_BUILD_TYPE=Release",
            "CMAKE_FIND_USE_PACKAGE_REGISTRY=OFF",
            "CMAKE_FIND_USE_SYSTEM_PACKAGE_REGISTRY=OFF",
            "CMAKE_SKIP_RPATH=TRUE",
            "MEMPROF=OFF",
            "OPENSSL_USE_STATIC_LIBS=TRUE",
            "TD_ENABLE_DOTNET=OFF",
            "TD_ENABLE_JNI=OFF",
            "TD_ENABLE_LTO=OFF",
            "TD_INSTALL_SHARED_LIBRARIES=ON",
            "TD_INSTALL_STATIC_LIBRARIES=OFF",
        ],
        "dynamic_dependencies": [
            "libz.so.1",
            "libstdc++.so.6",
            "libm.so.6",
            "libc.so.6",
            "ld-linux-x86-64.so.2",
        ],
    },
}


def load_linux_contracts() -> tuple[dict[str, Any], dict[str, Any]]:
    policy = read_json_bounded(LINUX_POLICY_PATH, MAX_POLICY_BYTES, "Linux policy")
    errors = exact_contract_errors(policy, EXPECTED_LINUX_POLICY, "Linux policy")
    source_policy = read_json_bounded(POLICY_PATH, MAX_POLICY_BYTES, "native policy")
    source = source_policy.get("source")
    if not isinstance(source, dict):
        errors.append("native policy source contract is absent")
    elif canonical_sha256(source) != policy["source_contract_sha256"]:
        errors.append("Linux source contract differs from canonical native source")
    if errors:
        raise NativeBuildError("; ".join(errors))
    return policy, source


def linux_recipe_fingerprints() -> dict[str, str]:
    result: dict[str, str] = {}
    for relative in LINUX_RECIPE_PATHS:
        digest, _ = sha256_file(ROOT / relative, 512 * 1024, f"Linux recipe {relative}")
        result[relative] = digest
    return result


def expected_linux_target_record(policy: dict[str, Any]) -> dict[str, Any]:
    target = policy["target"]
    return {
        "triple": target["triple"],
        "cmake_generator": target["cmake_generator"],
        "cmake_target": target["cmake_target"],
        "parallel_jobs": policy["limits"]["cpus"],
        "cmake_defines": target["cmake_defines"],
    }


def _phase_errors(name: str, phase: object, policy: dict[str, Any]) -> list[str]:
    expected = {
        "duration_seconds",
        "log_retained",
        "log_bytes",
        "log_sha256",
        "return_code",
    }
    if not isinstance(phase, dict) or set(phase) != expected:
        return [f"Linux provenance phase {name} has invalid closed schema"]
    limit = policy["limits"][f"{name}_seconds"]
    errors: list[str] = []
    duration = phase.get("duration_seconds")
    if isinstance(duration, bool) or not isinstance(duration, (int, float)) or not 0 <= duration <= limit + 2:
        errors.append(f"Linux provenance phase {name} duration exceeds cap")
    retained = phase.get("log_retained")
    log_bytes = phase.get("log_bytes")
    digest = phase.get("log_sha256")
    if retained is True:
        if (
            isinstance(log_bytes, bool)
            or not isinstance(log_bytes, int)
            or not 0 < log_bytes <= 16 * 1024**2
        ):
            errors.append(f"Linux provenance phase {name} log bytes are invalid")
        if (
            not isinstance(digest, str)
            or len(digest) != 64
            or any(c not in "0123456789abcdef" for c in digest)
        ):
            errors.append(f"Linux provenance phase {name} log hash is invalid")
    elif retained is False:
        if log_bytes is not None or digest is not None:
            errors.append(f"Linux provenance phase {name} discarded log metadata differs")
    else:
        errors.append(f"Linux provenance phase {name} log retention marker is invalid")
    if phase.get("return_code") != 0:
        errors.append(f"Linux provenance phase {name} failed")
    return errors


def linux_provenance_errors(
    provenance: dict[str, Any], policy: dict[str, Any], source: dict[str, Any]
) -> list[str]:
    expected_top = {
        "format_version", "source", "policy_sha256", "reviewed_recipe", "target",
        "build", "artifact", "verification", "reproducibility",
    }
    errors: list[str] = []
    if set(provenance) != expected_top:
        errors.append("Linux provenance top-level fields differ")
    if provenance.get("format_version") != 1:
        errors.append("Linux provenance format_version must be 1")
    if provenance.get("source") != source:
        errors.append("Linux provenance source differs from canonical source")
    if provenance.get("policy_sha256") != canonical_sha256(policy):
        errors.append("Linux provenance policy hash differs")
    if provenance.get("reviewed_recipe") != {"files": linux_recipe_fingerprints()}:
        errors.append("Linux provenance reviewed recipe differs from current recipe")
    if provenance.get("target") != expected_linux_target_record(policy):
        errors.append("Linux provenance target differs")

    build = provenance.get("build")
    expected_build = {"built_at_utc", "executor", "builder", "source_preparation", "phases"}
    if not isinstance(build, dict) or set(build) != expected_build:
        errors.append("Linux provenance build has invalid closed schema")
    else:
        try:
            timestamp = datetime.fromisoformat(str(build["built_at_utc"]).replace("Z", "+00:00"))
            if timestamp.tzinfo is None:
                raise ValueError("timezone absent")
        except ValueError:
            errors.append("Linux provenance timestamp is invalid")
        builder = build.get("builder")
        expected_builder = {
            "image_id": policy["builder"]["image_id"],
            "platform": policy["builder"]["platform"],
            "packages": policy["builder"]["packages"],
        }
        if builder != expected_builder:
            errors.append("Linux provenance builder differs from exact policy")
        executor = build.get("executor")
        if not isinstance(executor, dict) or set(executor) != {"system", "machine", "docker_client", "docker_server"}:
            errors.append("Linux provenance executor has invalid closed schema")
        elif any(not isinstance(value, str) or not value for value in executor.values()):
            errors.append("Linux provenance executor value is invalid")
        preparation = build.get("source_preparation")
        expected_preparation = {
            "archive_sha256", "archive_bytes", "archive_cache_reused",
            "extracted_logical_bytes", "extracted_members", "commit_identity",
        }
        if not isinstance(preparation, dict) or set(preparation) != expected_preparation:
            errors.append("Linux provenance source preparation has invalid schema")
        else:
            if (
                preparation.get("archive_sha256") != source["archive_sha256"]
                or preparation.get("archive_bytes") != source["archive_bytes"]
            ):
                errors.append("Linux provenance source archive differs")
            if not isinstance(preparation.get("archive_cache_reused"), bool):
                errors.append("Linux provenance archive reuse marker is invalid")
            extracted_bytes = preparation.get("extracted_logical_bytes")
            if (
                isinstance(extracted_bytes, bool)
                or not isinstance(extracted_bytes, int)
                or not 0 < extracted_bytes <= policy["limits"]["extracted_source_bytes"]
            ):
                errors.append("Linux provenance extracted source bytes are invalid")
            extracted_members = preparation.get("extracted_members")
            if (
                isinstance(extracted_members, bool)
                or not isinstance(extracted_members, int)
                or not 0
                < extracted_members
                <= policy["limits"]["source_archive_members"]
            ):
                errors.append("Linux provenance extracted member count is invalid")
            if preparation.get("commit_identity") != {
                "strategy": source["commit_identity_strategy"],
                "commit": source["commit"],
                "head_sha256": source["git_head_sha256"],
                "template_sha256": source["git_commit_template_sha256"],
                "generated_sha256": source["git_commit_generated_sha256"],
            }:
                errors.append("Linux provenance commit identity differs")
        phases = build.get("phases")
        if not isinstance(phases, dict) or set(phases) != {"configure", "build"}:
            errors.append("Linux provenance phases differ")
        else:
            errors.extend(_phase_errors("configure", phases["configure"], policy))
            errors.extend(_phase_errors("build", phases["build"], policy))

    artifact = provenance.get("artifact")
    if not isinstance(artifact, dict) or set(artifact) != {
        "cache_path",
        "sha256",
        "bytes",
    }:
        errors.append("Linux provenance artifact has invalid closed schema")
    else:
        digest = artifact.get("sha256")
        if (
            not isinstance(digest, str)
            or len(digest) != 64
            or any(c not in "0123456789abcdef" for c in digest)
        ):
            errors.append("Linux artifact hash is invalid")
        else:
            expected_path = str(
                Path(policy["target"]["artifact_cache_directory"])
                / digest
                / policy["target"]["artifact_basename"]
            )
            if artifact.get("cache_path") != expected_path:
                errors.append("Linux artifact cache path differs")
        size = artifact.get("bytes")
        if (
            isinstance(size, bool)
            or not isinstance(size, int)
            or not 0 < size <= policy["limits"]["artifact_bytes"]
        ):
            errors.append("Linux artifact bytes are invalid")

    verification = provenance.get("verification")
    expected_verification = {
        "file_format", "elf", "soname", "rpaths", "dynamic_dependencies",
        "exports", "options", "database_files_created", "glibc",
    }
    if not isinstance(verification, dict) or set(verification) != expected_verification:
        errors.append("Linux provenance verification has invalid schema")
    else:
        if "ELF 64-bit" not in str(
            verification.get("file_format")
        ) or "x86-64" not in str(verification.get("file_format")):
            errors.append("Linux artifact is not ELF x86-64")
        if verification.get("elf") != {
            "class": "ELF64", "data": "2's complement, little endian",
            "type": "DYN", "machine": "Advanced Micro Devices X86-64",
        }:
            errors.append("Linux ELF identity differs")
        if (
            verification.get("soname") != policy["target"]["artifact_basename"]
            or verification.get("rpaths") != []
        ):
            errors.append("Linux SONAME/RPATH contract differs")
        if verification.get("dynamic_dependencies") != policy["target"]["dynamic_dependencies"]:
            errors.append("Linux dynamic dependency inventory differs")
        if verification.get("exports") != list(LINUX_REQUIRED_EXPORTS):
            errors.append("Linux required exports differ")
        if verification.get("options") != {"version": source["version"], "commit_hash": source["commit"]}:
            errors.append("Linux TDJSON runtime identity differs")
        if verification.get("database_files_created") != 0:
            errors.append("Linux no-client smoke created database files")
        if not str(verification.get("glibc", "")).startswith("ldd (Debian GLIBC 2.36"):
            errors.append("Linux glibc builder identity differs")

    expected_reproducibility = {
        "status": "not_verified",
        "independent_builds": 1,
        "claim": "exact source and pinned builder observation only; bit-for-bit reproducibility is not established",
    }
    if provenance.get("reproducibility") != expected_reproducibility:
        errors.append("Linux provenance reproducibility overclaims or differs")
    return errors


def _docker_capture(command: Sequence[str], timeout: int) -> str:
    try:
        result = subprocess.run(
            list(command), check=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, timeout=timeout,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise NativeBuildError(f"Docker command failed to start: {command[1]}") from error
    if result.returncode != 0:
        tail = result.stderr.strip().splitlines()[-1:] or ["no stderr"]
        raise NativeBuildError(f"Docker command failed: {command[1]}: {tail[0]}")
    if len(result.stdout.encode("utf-8")) > 2 * 1024 * 1024:
        raise NativeBuildError("Docker command output exceeds cap")
    return result.stdout.strip()


def verify_builder_image(policy: dict[str, Any]) -> None:
    image = policy["builder"]["image_id"]
    actual = _docker_capture(
        ["docker", "image", "inspect", "--format", "{{.Id}} {{.Architecture}} {{.Os}}", image],
        20,
    )
    if actual != f"{image} amd64 linux":
        raise NativeBuildError(f"Linux builder image identity differs: {actual}")


def inspect_linux_artifact(
    artifact: Path, policy: dict[str, Any], source: dict[str, Any]
) -> dict[str, Any]:
    digest, size = sha256_file(artifact, policy["limits"]["artifact_bytes"], "Linux artifact")
    verify_builder_image(policy)
    name = f"telegram-cli-tdlib-inspect-{os.getpid()}-{secrets.token_hex(4)}"
    image = policy["builder"]["image_id"]
    create = [
        "docker", "create", "--name", name, "--platform", policy["builder"]["platform"],
        "--network", "none", "--memory", "1g", "--pids-limit", "32", image,
        "python3", "/inspect.py", "--artifact", "/artifact.so",
        "--expected-version", source["version"], "--expected-commit", source["commit"],
        "--expected-soname", policy["target"]["artifact_basename"],
        "--smoke-script", "/smoke.py",
    ]
    _docker_capture(create, 30)
    try:
        for local, remote in (
            (artifact, "/artifact.so"),
            (ROOT / "scripts/inspect-tdlib-linux-native.py", "/inspect.py"),
            (ROOT / "scripts/smoke-tdlib-native.py", "/smoke.py"),
        ):
            _docker_capture(["docker", "cp", str(local), f"{name}:{remote}"], 30)
        raw = _docker_capture(["docker", "start", "--attach", name], 60)
        verification = json.loads(raw)
        if not isinstance(verification, dict):
            raise NativeBuildError("Linux inspection did not return an object")
    except json.JSONDecodeError as error:
        raise NativeBuildError("Linux inspection returned invalid JSON") from error
    finally:
        subprocess.run(
            ["docker", "rm", "-f", name], check=False,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=30,
        )
    return {"sha256": digest, "bytes": size, "verification": verification}


def linux_local_artifact_errors(
    provenance: dict[str, Any], inspection: dict[str, Any]
) -> list[str]:
    errors: list[str] = []
    if provenance.get("artifact", {}).get("sha256") != inspection.get("sha256"):
        errors.append("local Linux artifact hash differs from provenance")
    if provenance.get("artifact", {}).get("bytes") != inspection.get("bytes"):
        errors.append("local Linux artifact bytes differ from provenance")
    if provenance.get("verification") != inspection.get("verification"):
        errors.append("local Linux artifact verification differs from provenance")
    return errors


def linux_artifact_cache_path(policy: dict[str, Any], digest: str) -> Path:
    return artifact_cache_path(policy, digest)


def linux_executor_record() -> dict[str, str]:
    client = _docker_capture(["docker", "version", "--format", "{{.Client.Version}}"], 20)
    server = _docker_capture(["docker", "version", "--format", "{{.Server.Version}}"], 20)
    return {
        "system": platform.system(),
        "machine": platform.machine(),
        "docker_client": client,
        "docker_server": server,
    }
