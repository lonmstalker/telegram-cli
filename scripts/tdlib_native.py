#!/usr/bin/env python3
"""Общий exact contract и resource guards для native TDLib artifact.

Модуль использует только stdlib. Он намеренно не загружает `.env.local`, не
наследует окружение для дочерних процессов и не публикует непроверенные файлы.
"""

from __future__ import annotations

from contextlib import contextmanager
from dataclasses import dataclass
import copy
from datetime import datetime, timezone
import errno
import fcntl
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import signal
import socket
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
import secrets
from typing import Any, Iterator, Mapping, Sequence


ROOT = Path(__file__).resolve().parent.parent
POLICY_PATH = ROOT / "vendor/tdlib/native-build-policy.json"
SCHEMA_MANIFEST_PATH = ROOT / "vendor/tdlib/manifest.json"
PROVENANCE_PATH = ROOT / "vendor/tdlib/native-builds/aarch64-apple-darwin.json"
NATIVE_ROOT = ROOT / "target/tdlib-native"
LOCK_PATH = NATIVE_ROOT / ".build.lock"
WORK_PREFIX = ".work-"
REAP_PREFIX = ".reap-"
REAP_PROOF_PREFIX = ".reap-proof-"
REAP_PROOF_SUFFIX = ".json"
WORK_MARKER = ".owner.json"
WORK_ID_HEX_BYTES = 16
MAX_POLICY_BYTES = 32 * 1024
MAX_SCHEMA_MANIFEST_BYTES = 16 * 1024
MAX_PROVENANCE_BYTES = 128 * 1024
MAX_RECIPE_FILE_BYTES = 512 * 1024
MAX_INSPECTION_OUTPUT_BYTES = 2 * 1024 * 1024

REQUIRED_EXPORTS = (
    "td_json_client_create",
    "td_json_client_send",
    "td_json_client_receive",
    "td_json_client_execute",
    "td_json_client_destroy",
)
RECIPE_PATHS = (
    "scripts/tdlib_native.py",
    "scripts/build-tdlib-native.py",
    "scripts/process-group-watchdog.py",
    "scripts/process-group-target-gate.py",
    "scripts/smoke-tdlib-native.py",
)

EXPECTED_POLICY: dict[str, Any] = {
    "format_version": 1,
    "source": {
        "repository": "https://github.com/tdlib/td",
        "commit": "07d3a0973f5113b0827a04d54a93aaaa9e288348",
        "version": "1.8.66",
        "schema_sha256": (
            "10a00b48d557d00c0daa231a8dad38a9d0c99de78360a1e4b0b7579b28188f31"
        ),
        "archive_url": (
            "https://github.com/tdlib/td/archive/"
            "07d3a0973f5113b0827a04d54a93aaaa9e288348.tar.gz"
        ),
        "archive_sha256": (
            "1a8c9429af7b2eae2e36f24db9a9b8f523dbd5bd2ae448f8bb9322bb1c80dbfb"
        ),
        "archive_bytes": 5_785_154,
        "archive_root": "td-07d3a0973f5113b0827a04d54a93aaaa9e288348",
        "content_length_policy": "verify-if-present",
        "commit_identity_strategy": "synthetic-detached-head",
        "git_head_sha256": (
            "037fbbb82c16d37a78e24bdf0a0d16075d0cb217da1f6f74b45e53df9e7a08a7"
        ),
        "git_commit_template_sha256": (
            "5acbcbb038b97ec3a05cf04c3b2e50cf7f846fdcfb03cdbe4a9d1ce6a7b3e1e7"
        ),
        "git_commit_generated_sha256": (
            "27d3e57adf55d87170b9390c54d287703d043094c84a78bdcf2e756aa0c30c0b"
        ),
    },
    "limits": {
        "parallel_jobs": 2,
        "download_seconds": 120,
        "source_archive_bytes": 8 * 1024 * 1024,
        "source_archive_members": 50_000,
        "source_archive_member_bytes": 16 * 1024 * 1024,
        "source_archive_path_bytes": 512,
        "source_archive_path_depth": 32,
        "extracted_source_bytes": 256 * 1024 * 1024,
        "build_tree_bytes": 4 * 1024 * 1024 * 1024,
        "process_group_rss_bytes": 8 * 1024 * 1024 * 1024,
        "process_group_processes": 16,
        "log_bytes": 16 * 1024 * 1024,
        "configure_seconds": 900,
        "build_seconds": 5_400,
        "artifact_bytes": 64 * 1024 * 1024,
        "cached_artifacts": 1,
        "inspection_seconds": 20,
        "inspection_rss_bytes": 512 * 1024 * 1024,
        "termination_grace_seconds": 5,
        "build_lock_wait_seconds": 30,
        "stale_cleanup_seconds": 60,
        "stale_cleanup_entries": 500_000,
        "stale_work_directories": 2,
        "stale_guard_states": 2,
    },
    "target": {
        "triple": "aarch64-apple-darwin",
        "cmake_generator": "Unix Makefiles",
        "cmake_target": "tdjson",
        "artifact_basename": "libtdjson.1.8.66.dylib",
        "artifact_cache_directory": (
            "target/tdlib-native/aarch64-apple-darwin/by-sha256"
        ),
        "cmake_defines": [
            "BUILD_TESTING=OFF",
            "CCACHE_FOUND=CCACHE_FOUND-NOTFOUND",
            "CMAKE_BUILD_TYPE=Release",
            "CMAKE_FIND_USE_PACKAGE_REGISTRY=OFF",
            "CMAKE_FIND_USE_SYSTEM_PACKAGE_REGISTRY=OFF",
            "CMAKE_OSX_ARCHITECTURES=arm64",
            "CMAKE_OSX_DEPLOYMENT_TARGET=11.0",
            "MEMPROF=OFF",
            "OPENSSL_USE_STATIC_LIBS=TRUE",
            "TD_ENABLE_DOTNET=OFF",
            "TD_ENABLE_JNI=OFF",
            "TD_ENABLE_LTO=OFF",
            "TD_INSTALL_SHARED_LIBRARIES=ON",
            "TD_INSTALL_STATIC_LIBRARIES=OFF",
        ],
        "dependencies": {
            "openssl": {
                "provider": "homebrew",
                "formula": "openssl@3",
                "linkage": "static",
            },
            "zlib": {"provider": "macos-sdk", "linkage": "system"},
        },
    },
}


class NativeBuildError(RuntimeError):
    """Fail-closed ошибка native build contract."""


class BuildGuardError(NativeBuildError):
    """Команда нарушила ресурсный guard или завершилась ошибкой."""

    def __init__(self, message: str, process_group_id: int, metrics: "GuardMetrics"):
        super().__init__(message)
        self.process_group_id = process_group_id
        self.metrics = metrics


@dataclass(frozen=True, slots=True)
class BuildLease:
    """Удерживаемый global build lock, наследуемый только watchdog-процессом."""

    path: Path
    descriptor: int


@dataclass(frozen=True, slots=True)
class OwnedWorkDirectory:
    """Private scratch с durable identity marker."""

    path: Path
    work_id: str


class GuardMetrics:
    """Измерения одной ограниченной process group."""

    __slots__ = (
        "process_group_id",
        "duration_seconds",
        "peak_tree_bytes",
        "peak_group_rss_bytes",
        "peak_group_processes",
        "log_bytes",
        "return_code",
    )

    def __init__(
        self,
        *,
        process_group_id: int,
        duration_seconds: float,
        peak_tree_bytes: int,
        peak_group_rss_bytes: int,
        peak_group_processes: int,
        log_bytes: int,
        return_code: int,
    ) -> None:
        self.process_group_id = process_group_id
        self.duration_seconds = duration_seconds
        self.peak_tree_bytes = peak_tree_bytes
        self.peak_group_rss_bytes = peak_group_rss_bytes
        self.peak_group_processes = peak_group_processes
        self.log_bytes = log_bytes
        self.return_code = return_code

    def provenance_record(self, log_sha256: str) -> dict[str, Any]:
        return {
            "duration_seconds": round(self.duration_seconds, 3),
            "peak_tree_bytes": self.peak_tree_bytes,
            "peak_group_rss_bytes": self.peak_group_rss_bytes,
            "peak_group_processes": self.peak_group_processes,
            "log_bytes": self.log_bytes,
            "log_sha256": log_sha256,
            "return_code": self.return_code,
        }


def read_bounded(path: Path, maximum_bytes: int, label: str) -> bytes:
    """Читает regular file без symlink и проверяет неизменность размера."""
    metadata = path.lstat()
    if not stat.S_ISREG(metadata.st_mode):
        raise ValueError(f"{label}: expected regular file without symlink: {path}")
    if metadata.st_size > maximum_bytes:
        raise ValueError(
            f"{label}: file exceeds hard cap {maximum_bytes}: {metadata.st_size}"
        )
    flags = os.O_RDONLY
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    descriptor = os.open(path, flags)
    try:
        opened = os.fstat(descriptor)
        if not stat.S_ISREG(opened.st_mode) or opened.st_ino != metadata.st_ino:
            raise ValueError(f"{label}: file changed before bounded open")
        chunks: list[bytes] = []
        remaining = maximum_bytes + 1
        while remaining:
            chunk = os.read(descriptor, min(1024 * 1024, remaining))
            if not chunk:
                break
            chunks.append(chunk)
            remaining -= len(chunk)
        payload = b"".join(chunks)
        after = os.fstat(descriptor)
    finally:
        os.close(descriptor)
    if len(payload) > maximum_bytes:
        raise ValueError(f"{label}: bounded read exceeded hard cap {maximum_bytes}")
    if len(payload) != opened.st_size or after.st_size != opened.st_size:
        raise ValueError(f"{label}: file changed during bounded read")
    return payload


def read_json_bounded(path: Path, maximum_bytes: int, label: str) -> dict[str, Any]:
    try:
        value = json.loads(read_bounded(path, maximum_bytes, label).decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError(f"{label}: invalid UTF-8 JSON: {error}") from error
    if not isinstance(value, dict):
        raise ValueError(f"{label}: expected JSON object")
    return value


def canonical_sha256(value: Any) -> str:
    encoded = json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def sha256_file(path: Path, maximum_bytes: int, label: str) -> tuple[str, int]:
    metadata = path.lstat()
    if not stat.S_ISREG(metadata.st_mode):
        raise ValueError(f"{label}: expected regular file without symlink: {path}")
    if metadata.st_size > maximum_bytes:
        raise ValueError(
            f"{label}: file exceeds hard cap {maximum_bytes}: {metadata.st_size}"
        )
    flags = os.O_RDONLY
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    descriptor = os.open(path, flags)
    digest = hashlib.sha256()
    total = 0
    try:
        opened = os.fstat(descriptor)
        if opened.st_ino != metadata.st_ino or not stat.S_ISREG(opened.st_mode):
            raise ValueError(f"{label}: file changed before hash")
        while True:
            chunk = os.read(descriptor, 1024 * 1024)
            if not chunk:
                break
            total += len(chunk)
            if total > maximum_bytes:
                raise ValueError(f"{label}: hash exceeded hard cap {maximum_bytes}")
            digest.update(chunk)
        after = os.fstat(descriptor)
    finally:
        os.close(descriptor)
    if total != opened.st_size or after.st_size != opened.st_size:
        raise ValueError(f"{label}: file changed during hash")
    return digest.hexdigest(), total


def _flatten(value: Any, prefix: str = "") -> dict[str, Any]:
    if not isinstance(value, dict):
        return {prefix: value}
    flattened: dict[str, Any] = {}
    for key, item in value.items():
        path = f"{prefix}.{key}" if prefix else str(key)
        flattened.update(_flatten(item, path))
    return flattened


def exact_contract_errors(
    actual: dict[str, Any], expected: dict[str, Any], label: str
) -> list[str]:
    if actual == expected:
        return []
    actual_fields = _flatten(actual)
    expected_fields = _flatten(expected)
    errors: list[str] = []
    for field in sorted(set(actual_fields) | set(expected_fields)):
        actual_value = actual_fields.get(field, "<missing>")
        expected_value = expected_fields.get(field, "<missing>")
        if actual_value != expected_value:
            errors.append(
                f"{label}.{field}: ожидалось {expected_value!r}, "
                f"получено {actual_value!r}"
            )
    if not errors:
        errors.append(f"{label}: structure differs from exact contract")
    return errors


def validate_policy_contract(
    policy: dict[str, Any], schema_manifest: dict[str, Any]
) -> list[str]:
    errors = exact_contract_errors(policy, EXPECTED_POLICY, "policy")
    source = policy.get("source", {})
    upstream = schema_manifest.get("upstream", {})
    schema = schema_manifest.get("schema", {})
    cross_checks = {
        "repository": upstream.get("repository"),
        "commit": upstream.get("commit"),
        "version": upstream.get("version"),
        "schema_sha256": schema.get("sha256"),
    }
    for field, actual in cross_checks.items():
        if source.get(field) != actual:
            errors.append(
                f"policy.source.{field} disagrees with schema manifest: {actual!r}"
            )

    limits = policy.get("limits", {})
    for field, value in limits.items():
        if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
            errors.append(f"policy.limits.{field}: expected positive integer")
    archive_bytes = source.get("archive_bytes")
    archive_cap = limits.get("source_archive_bytes")
    if (
        isinstance(archive_bytes, int)
        and isinstance(archive_cap, int)
        and archive_bytes > archive_cap
    ):
        errors.append("policy.source.archive_bytes exceeds source archive cap")

    cache_value = policy.get("target", {}).get("artifact_cache_directory", "")
    cache_path = Path(str(cache_value))
    if (
        cache_path.is_absolute()
        or ".." in cache_path.parts
        or cache_path.parts[:2] != ("target", "tdlib-native")
    ):
        errors.append(
            "policy.target.artifact_cache_directory is outside ignored native root"
        )
    return errors


def load_exact_contracts() -> tuple[dict[str, Any], dict[str, Any]]:
    policy = read_json_bounded(POLICY_PATH, MAX_POLICY_BYTES, "native policy")
    schema_manifest = read_json_bounded(
        SCHEMA_MANIFEST_PATH, MAX_SCHEMA_MANIFEST_BYTES, "schema manifest"
    )
    errors = validate_policy_contract(policy, schema_manifest)
    if errors:
        raise NativeBuildError("; ".join(errors))
    return policy, schema_manifest


def recipe_fingerprints() -> dict[str, str]:
    fingerprints: dict[str, str] = {}
    for relative in RECIPE_PATHS:
        digest, _ = sha256_file(ROOT / relative, MAX_RECIPE_FILE_BYTES, relative)
        fingerprints[relative] = digest
    return fingerprints


def atomic_write(path: Path, payload: bytes, mode: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary_path: Path | None = None
    try:
        descriptor, name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
        temporary_path = Path(name)
        with os.fdopen(descriptor, "wb") as output:
            output.write(payload)
            output.flush()
            os.fsync(output.fileno())
        temporary_path.chmod(mode)
        os.replace(temporary_path, path)
        directory_descriptor = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(directory_descriptor)
        finally:
            os.close(directory_descriptor)
    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)


def atomic_copy(path: Path, source: Path, maximum_bytes: int) -> tuple[str, int]:
    expected_digest, expected_bytes = sha256_file(source, maximum_bytes, "artifact")
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary_path: Path | None = None
    try:
        descriptor, name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
        temporary_path = Path(name)
        with source.open("rb") as input_file, os.fdopen(descriptor, "wb") as output:
            copied = 0
            while True:
                chunk = input_file.read(1024 * 1024)
                if not chunk:
                    break
                copied += len(chunk)
                if copied > maximum_bytes:
                    raise NativeBuildError("artifact copy exceeded hard cap")
                output.write(chunk)
            output.flush()
            os.fsync(output.fileno())
        temporary_path.chmod(0o555)
        actual_digest, actual_bytes = sha256_file(
            temporary_path, maximum_bytes, "staged artifact"
        )
        if (actual_digest, actual_bytes) != (expected_digest, expected_bytes):
            raise NativeBuildError("artifact changed during atomic staging")
        os.replace(temporary_path, path)
        directory_descriptor = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(directory_descriptor)
        finally:
            os.close(directory_descriptor)
    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)
    return expected_digest, expected_bytes


@contextmanager
def exclusive_build_lock(
    path: Path = LOCK_PATH, *, wait_seconds: float = 0.0
) -> Iterator[BuildLease]:
    if isinstance(wait_seconds, bool) or wait_seconds < 0:
        raise ValueError("build lock wait must be a non-negative number")
    path.parent.mkdir(parents=True, exist_ok=True)
    flags = os.O_CREAT | os.O_RDWR
    if hasattr(os, "O_CLOEXEC"):
        flags |= os.O_CLOEXEC
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    descriptor = os.open(path, flags, 0o600)
    try:
        opened = os.fstat(descriptor)
        if not stat.S_ISREG(opened.st_mode):
            raise NativeBuildError("native build lock is not a regular file")
        os.fchmod(descriptor, 0o600)
        deadline = time.monotonic() + wait_seconds
        while True:
            try:
                fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError as error:
                if time.monotonic() >= deadline:
                    raise NativeBuildError(
                        "другая native TDLib build или cleanup watchdog владеет lock"
                    ) from error
                time.sleep(min(0.05, max(0.0, deadline - time.monotonic())))
        os.ftruncate(descriptor, 0)
        os.write(descriptor, f"pid={os.getpid()}\n".encode("ascii"))
        os.fsync(descriptor)
        yield BuildLease(path=path.resolve(strict=True), descriptor=descriptor)
    finally:
        try:
            fcntl.flock(descriptor, fcntl.LOCK_UN)
        finally:
            os.close(descriptor)


@contextmanager
def shared_artifact_lock(path: Path = LOCK_PATH) -> Iterator[None]:
    """Не даёт local checker читать пару artifact/provenance во время publish."""
    path.parent.mkdir(parents=True, exist_ok=True)
    flags = os.O_CREAT | os.O_RDONLY
    if hasattr(os, "O_CLOEXEC"):
        flags |= os.O_CLOEXEC
    descriptor = os.open(path, flags, 0o600)
    try:
        try:
            fcntl.flock(descriptor, fcntl.LOCK_SH | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise NativeBuildError("native artifact is being published") from error
        yield
    finally:
        try:
            fcntl.flock(descriptor, fcntl.LOCK_UN)
        finally:
            os.close(descriptor)


def _valid_work_id(value: object) -> bool:
    return (
        isinstance(value, str)
        and len(value) == WORK_ID_HEX_BYTES * 2
        and all(character in "0123456789abcdef" for character in value)
    )


def _work_id_from_name(name: str, prefix: str) -> str | None:
    if not name.startswith(prefix):
        return None
    candidate = name[len(prefix) :]
    return candidate if _valid_work_id(candidate) else None


def _work_id_from_proof_name(name: str) -> str | None:
    if not name.startswith(REAP_PROOF_PREFIX) or not name.endswith(
        REAP_PROOF_SUFFIX
    ):
        return None
    candidate = name[len(REAP_PROOF_PREFIX) : -len(REAP_PROOF_SUFFIX)]
    return candidate if _valid_work_id(candidate) else None


def _fsync_directory(path: Path) -> None:
    descriptor = os.open(path, os.O_RDONLY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def _assert_private_directory(path: Path, *, label: str) -> os.stat_result:
    metadata = path.lstat()
    if (
        not stat.S_ISDIR(metadata.st_mode)
        or metadata.st_uid != os.getuid()
        or stat.S_IMODE(metadata.st_mode) & 0o077
    ):
        raise NativeBuildError(f"{label} must be an owned private directory: {path}")
    return metadata


def _validate_marker_file(
    marker_path: Path, work_id: str, *, expected_relative_path: str
) -> dict[str, Any]:
    try:
        metadata = marker_path.lstat()
    except OSError as error:
        raise NativeBuildError(
            f"native scratch marker is missing or unreadable: {marker_path}"
        ) from error
    if (
        not stat.S_ISREG(metadata.st_mode)
        or metadata.st_uid != os.getuid()
        or stat.S_IMODE(metadata.st_mode) != 0o600
    ):
        raise NativeBuildError(f"native scratch marker is unsafe: {marker_path}")
    try:
        marker = read_json_bounded(marker_path, 4096, "native scratch marker")
    except (OSError, ValueError) as error:
        raise NativeBuildError(
            f"native scratch marker is malformed: {marker_path}"
        ) from error
    if set(marker) != {
        "format_version",
        "kind",
        "work_id",
        "relative_path",
        "owner_process_id",
        "created_at",
    }:
        raise NativeBuildError(f"native scratch marker fields differ: {marker_path}")
    owner_process_id = marker.get("owner_process_id")
    if (
        marker.get("format_version") != 1
        or marker.get("kind") != "tdlib-native-work"
        or marker.get("work_id") != work_id
        or marker.get("relative_path") != expected_relative_path
        or isinstance(owner_process_id, bool)
        or not isinstance(owner_process_id, int)
        or owner_process_id <= 1
        or not isinstance(marker.get("created_at"), str)
        or not marker["created_at"].endswith("Z")
    ):
        raise NativeBuildError(f"native scratch marker values differ: {marker_path}")
    return marker


def _validate_work_marker(
    path: Path, work_id: str, *, expected_relative_path: str | None = None
) -> dict[str, Any]:
    return _validate_marker_file(
        path / WORK_MARKER,
        work_id,
        expected_relative_path=expected_relative_path or path.name,
    )


def _assert_build_lease(build_lease: BuildLease, native_root: Path) -> None:
    if not isinstance(build_lease, BuildLease) or build_lease.descriptor <= 2:
        raise NativeBuildError("stale cleanup requires an active global build lease")
    expected_path = native_root / ".build.lock"
    try:
        opened = os.fstat(build_lease.descriptor)
        on_disk = expected_path.lstat()
    except OSError as error:
        raise NativeBuildError("global build lease is not readable") from error
    if (
        build_lease.path != expected_path.resolve(strict=True)
        or not stat.S_ISREG(opened.st_mode)
        or (opened.st_dev, opened.st_ino) != (on_disk.st_dev, on_disk.st_ino)
    ):
        raise NativeBuildError("global build lease does not own native root")
    try:
        fcntl.flock(build_lease.descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError as error:
        raise NativeBuildError("global build lease is not held exclusively") from error


def _process_exists(process_id: int) -> bool:
    try:
        os.kill(process_id, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def _guard_states(
    path: Path,
    *,
    maximum_states: int,
    deadline: float,
    maximum_entries: int,
) -> tuple[list[Path], int]:
    states: list[Path] = []
    scanned = 0

    def visit(directory: Path, depth: int) -> None:
        nonlocal scanned
        if depth > 128:
            raise NativeBuildError(f"native scratch state scan depth exceeded: {directory}")
        with os.scandir(directory) as entries:
            for entry in entries:
                if time.monotonic() > deadline:
                    raise NativeBuildError("native scratch state scan timeout exceeded")
                scanned += 1
                if scanned > maximum_entries:
                    raise NativeBuildError(
                        f"native scratch state scan entry cap exceeded: {maximum_entries}"
                    )
                child = Path(entry.path)
                metadata = child.lstat()
                if stat.S_ISDIR(metadata.st_mode):
                    visit(child, depth + 1)
                elif entry.name.startswith(".") and entry.name.endswith(
                    ".guard-state.json"
                ):
                    states.append(child)
                    if len(states) > maximum_states:
                        raise NativeBuildError(
                            f"native scratch has more than {maximum_states} guard states"
                        )

    visit(path, 0)
    return states, scanned


def _validate_inactive_guard_state(path: Path, work_id: str) -> None:
    try:
        metadata = path.lstat()
        if (
            not stat.S_ISREG(metadata.st_mode)
            or metadata.st_uid != os.getuid()
            or stat.S_IMODE(metadata.st_mode) != 0o600
        ):
            raise NativeBuildError(f"stale watchdog state is unsafe: {path}")
        state = read_json_bounded(path, 4096, "stale watchdog state")
    except NativeBuildError:
        raise
    except (OSError, ValueError) as error:
        raise NativeBuildError(f"stale watchdog state is malformed: {path}") from error
    expected_fields = {
        "format_version",
        "work_id",
        "phase",
        "guard_parent_process_id",
        "watchdog_process_id",
        "target_process_id",
        "target_process_group_id",
    }
    if set(state) != expected_fields or state.get("format_version") != 2:
        raise NativeBuildError(f"stale watchdog state schema differs: {path}")
    if state.get("work_id") != work_id or state.get("phase") not in {
        "starting",
        "running",
        "cleaning",
    }:
        raise NativeBuildError(f"stale watchdog state ownership differs: {path}")
    for field in (
        "guard_parent_process_id",
        "watchdog_process_id",
    ):
        value = state.get(field)
        if isinstance(value, bool) or not isinstance(value, int) or value <= 1:
            raise NativeBuildError(f"stale watchdog state {field} is invalid: {path}")
    for field in ("target_process_id", "target_process_group_id"):
        value = state.get(field)
        if value is not None and (
            isinstance(value, bool) or not isinstance(value, int) or value <= 1
        ):
            raise NativeBuildError(f"stale watchdog state {field} is invalid: {path}")
    if state["phase"] in {"running", "cleaning"} and (
        state["target_process_id"] is None
        or state["target_process_group_id"] is None
    ):
        raise NativeBuildError(f"stale watchdog state lacks running target: {path}")
    if _process_exists(state["watchdog_process_id"]):
        raise NativeBuildError(f"stale scratch watchdog is still alive: {path}")
    target_group = state["target_process_group_id"]
    if target_group is not None and _group_exists(target_group):
        raise NativeBuildError(f"stale scratch target group is still alive: {path}")


def _remove_reap_tree(
    path: Path, *, deadline: float, maximum_entries: int
) -> int:
    work_id = _work_id_from_name(path.name, REAP_PREFIX)
    if work_id is None:
        raise NativeBuildError(f"native reap name is invalid: {path}")
    expected_relative_path = f"{WORK_PREFIX}{work_id}"
    _validate_work_marker(
        path, work_id, expected_relative_path=expected_relative_path
    )
    proof_path = path.parent / (
        f"{REAP_PROOF_PREFIX}{work_id}{REAP_PROOF_SUFFIX}"
    )
    if proof_path.exists() or proof_path.is_symlink():
        raise NativeBuildError(f"native reap proof already exists: {proof_path}")
    visited = 0

    def remove_directory(directory: Path, depth: int) -> None:
        nonlocal visited
        if depth > 128:
            raise NativeBuildError(f"native scratch cleanup depth exceeded: {directory}")
        with os.scandir(directory) as entries:
            for entry in entries:
                if directory == path and entry.name == WORK_MARKER:
                    continue
                if time.monotonic() > deadline:
                    raise NativeBuildError("native scratch cleanup timeout exceeded")
                visited += 1
                if visited > maximum_entries:
                    raise NativeBuildError(
                        f"native scratch cleanup entry cap exceeded: {maximum_entries}"
                    )
                child = Path(entry.path)
                metadata = child.lstat()
                if stat.S_ISDIR(metadata.st_mode):
                    remove_directory(child, depth + 1)
                    child.rmdir()
                elif stat.S_ISREG(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode):
                    child.unlink()
                else:
                    raise NativeBuildError(
                        f"unsupported entry blocks native scratch cleanup: {child}"
                    )

    remove_directory(path, 0)
    marker_path = path / WORK_MARKER
    if not marker_path.is_file() or marker_path.is_symlink():
        raise NativeBuildError(f"native reap marker disappeared: {marker_path}")
    marker_path.rename(proof_path)
    _fsync_directory(path)
    _fsync_directory(path.parent)
    _validate_marker_file(
        proof_path,
        work_id,
        expected_relative_path=expected_relative_path,
    )
    path.rmdir()
    _fsync_directory(path.parent)
    proof_path.unlink()
    _fsync_directory(path.parent)
    return visited


def _retire_work_directory(
    path: Path, *, deadline: float, maximum_entries: int
) -> int:
    native_root = path.parent
    work_id = _work_id_from_name(path.name, WORK_PREFIX)
    if work_id is None:
        raise NativeBuildError(f"native scratch name is invalid: {path}")
    _assert_private_directory(path, label="native scratch")
    _validate_work_marker(path, work_id)
    reap_path = native_root / f"{REAP_PREFIX}{work_id}"
    if reap_path.exists() or reap_path.is_symlink():
        raise NativeBuildError(f"native scratch reap path already exists: {reap_path}")
    path.rename(reap_path)
    _fsync_directory(native_root)
    _validate_work_marker(
        reap_path,
        work_id,
        expected_relative_path=f"{WORK_PREFIX}{work_id}",
    )
    removed_entries = _remove_reap_tree(
        reap_path, deadline=deadline, maximum_entries=maximum_entries
    )
    _fsync_directory(native_root)
    return removed_entries


@contextmanager
def owned_work_directory(
    native_root: Path,
    *,
    cleanup_seconds: float = 60.0,
    maximum_entries: int = 500_000,
    maximum_guard_states: int = 2,
) -> Iterator[OwnedWorkDirectory]:
    if (
        isinstance(cleanup_seconds, bool)
        or cleanup_seconds <= 0
        or isinstance(maximum_entries, bool)
        or maximum_entries <= 0
        or isinstance(maximum_guard_states, bool)
        or maximum_guard_states <= 0
    ):
        raise ValueError("native scratch cleanup limits must be positive")
    native_root = native_root.resolve(strict=True)
    _assert_private_directory(native_root, label="native root")
    work_id = secrets.token_hex(WORK_ID_HEX_BYTES)
    path = native_root / f"{WORK_PREFIX}{work_id}"
    path.mkdir(mode=0o700)
    marker = {
        "format_version": 1,
        "kind": "tdlib-native-work",
        "work_id": work_id,
        "relative_path": path.name,
        "owner_process_id": os.getpid(),
        "created_at": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
    }
    try:
        atomic_write(
            path / WORK_MARKER,
            (
                json.dumps(marker, sort_keys=True, separators=(",", ":")) + "\n"
            ).encode("utf-8"),
            0o600,
        )
    except BaseException:
        path.rmdir()
        raise
    _fsync_directory(native_root)
    try:
        yield OwnedWorkDirectory(path=path, work_id=work_id)
    finally:
        if path.exists() or path.is_symlink():
            deadline = time.monotonic() + cleanup_seconds
            states, scanned_entries = _guard_states(
                path,
                maximum_states=maximum_guard_states,
                deadline=deadline,
                maximum_entries=maximum_entries,
            )
            for state_path in states:
                _validate_inactive_guard_state(state_path, work_id)
            _retire_work_directory(
                path,
                deadline=deadline,
                maximum_entries=maximum_entries - scanned_entries,
            )


def cleanup_stale_work_directories(
    *,
    build_lease: BuildLease,
    native_root: Path,
    cleanup_seconds: float,
    maximum_directories: int,
    maximum_guard_states: int,
    maximum_entries: int,
) -> int:
    """Удаляет только доказанно stale private scratch до нового build."""
    numeric_limits = (
        cleanup_seconds,
        maximum_directories,
        maximum_guard_states,
        maximum_entries,
    )
    if any(isinstance(value, bool) or value <= 0 for value in numeric_limits):
        raise ValueError("stale native cleanup limits must be positive")
    native_root = native_root.resolve(strict=True)
    _assert_private_directory(native_root, label="native root")
    _assert_build_lease(build_lease, native_root)
    work_candidates: dict[str, Path] = {}
    reap_candidates: dict[str, Path] = {}
    reap_proofs: dict[str, Path] = {}
    with os.scandir(native_root) as entries:
        for entry in entries:
            path = Path(entry.path)
            if entry.name.startswith(REAP_PROOF_PREFIX):
                work_id = _work_id_from_proof_name(entry.name)
                if work_id is None:
                    raise NativeBuildError(
                        f"malformed native reap proof blocks build: {path}"
                    )
                reap_proofs[work_id] = path
            elif entry.name.startswith(REAP_PREFIX):
                work_id = _work_id_from_name(entry.name, REAP_PREFIX)
                if work_id is None:
                    raise NativeBuildError(
                        f"malformed stale native reap blocks build: {path}"
                    )
                reap_candidates[work_id] = path
            elif entry.name.startswith(WORK_PREFIX):
                work_id = _work_id_from_name(entry.name, WORK_PREFIX)
                if work_id is None:
                    raise NativeBuildError(
                        f"malformed stale native scratch blocks build: {path}"
                    )
                work_candidates[work_id] = path
    scratch_ids = set(work_candidates) | set(reap_candidates) | set(reap_proofs)
    if len(scratch_ids) > maximum_directories:
        raise NativeBuildError(
            f"stale native scratch count exceeds {maximum_directories}"
        )
    ambiguous = set(work_candidates) & set(reap_candidates)
    if ambiguous:
        raise NativeBuildError(
            "work/reap ambiguity blocks destructive recovery: "
            + ", ".join(sorted(ambiguous))
        )
    deadline = time.monotonic() + cleanup_seconds
    removed = 0
    removed_entries = 0

    for work_id in sorted(reap_proofs):
        proof_path = reap_proofs[work_id]
        if work_id in work_candidates:
            raise NativeBuildError(
                f"native reap proof conflicts with live work marker: {proof_path}"
            )
        _validate_marker_file(
            proof_path,
            work_id,
            expected_relative_path=f"{WORK_PREFIX}{work_id}",
        )
        reap_path = reap_candidates.pop(work_id, None)
        if reap_path is not None:
            _assert_private_directory(reap_path, label="stale native reap")
            with os.scandir(reap_path) as remaining:
                if next(remaining, None) is not None:
                    raise NativeBuildError(
                        f"native reap proof accompanies non-empty directory: {reap_path}"
                    )
            if time.monotonic() > deadline:
                raise NativeBuildError("native scratch cleanup timeout exceeded")
            reap_path.rmdir()
            _fsync_directory(native_root)
            removed += 1
        proof_path.unlink()
        _fsync_directory(native_root)

    for work_id, candidate in sorted(reap_candidates.items()):
        metadata = candidate.lstat()
        if not stat.S_ISDIR(metadata.st_mode):
            raise NativeBuildError(
                f"unsafe stale native scratch blocks build: {candidate}"
            )
        _assert_private_directory(candidate, label="stale native scratch")
        _validate_work_marker(
            candidate,
            work_id,
            expected_relative_path=f"{WORK_PREFIX}{work_id}",
        )
        removed_entries += _remove_reap_tree(
            candidate,
            deadline=deadline,
            maximum_entries=maximum_entries - removed_entries,
        )
        removed += 1

    for work_id, candidate in sorted(work_candidates.items()):
        metadata = candidate.lstat()
        if not stat.S_ISDIR(metadata.st_mode):
            raise NativeBuildError(
                f"unsafe stale native scratch blocks build: {candidate}"
            )
        _assert_private_directory(candidate, label="stale native scratch")
        marker = _validate_work_marker(candidate, work_id)
        if marker["owner_process_id"] == os.getpid():
            raise NativeBuildError("refusing to reap scratch owned by current process")
        states, scanned_entries = _guard_states(
            candidate,
            maximum_states=maximum_guard_states,
            deadline=deadline,
            maximum_entries=maximum_entries - removed_entries,
        )
        removed_entries += scanned_entries
        for state_path in states:
            _validate_inactive_guard_state(state_path, work_id)
        removed_entries += _retire_work_directory(
            candidate,
            deadline=deadline,
            maximum_entries=maximum_entries - removed_entries,
        )
        removed += 1
    return removed


def directory_tree_bytes(root: Path) -> int:
    """Logical st_size дерева без следования symlink; ошибки — fail-closed."""
    total = 0
    pending = [root]
    while pending:
        current = pending.pop()
        try:
            metadata = current.lstat()
        except FileNotFoundError:
            continue
        total += metadata.st_size
        if stat.S_ISDIR(metadata.st_mode):
            try:
                entries = os.scandir(current)
            except FileNotFoundError:
                continue
            with entries:
                for entry in entries:
                    pending.append(Path(entry.path))
        elif not (stat.S_ISREG(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode)):
            raise NativeBuildError(f"unsupported filesystem entry in build tree: {current}")
    return total


def _group_usage(process_group_id: int) -> tuple[int, int]:
    try:
        result = subprocess.run(
            ["/bin/ps", "-axo", "pid=,pgid=,rss="],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=5,
            env={"PATH": "/usr/bin:/bin", "LC_ALL": "C"},
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise NativeBuildError(f"cannot measure process group: {error}") from error
    if result.returncode != 0 or len(result.stdout) > 4 * 1024 * 1024:
        raise NativeBuildError("bounded ps process-group measurement failed")
    rss_bytes = 0
    processes = 0
    for raw_line in result.stdout.splitlines():
        fields = raw_line.split()
        if len(fields) != 3:
            continue
        try:
            _, group, rss_kib = map(int, fields)
        except ValueError:
            continue
        if group == process_group_id:
            processes += 1
            rss_bytes += rss_kib * 1024
    return rss_bytes, processes


def _group_exists(process_group_id: int) -> bool:
    try:
        os.killpg(process_group_id, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        # Darwin может кратко вернуть EPERM для уже reaped session group.
        # Fail-open здесь нельзя: считаем группу живой, пока bounded ps видит
        # хотя бы zombie; caller подождёт ESRCH либо завершится с ошибкой.
        _, processes = _group_usage(process_group_id)
        return processes > 0
    return True


def _terminate_orphan_group(process_group_id: int, grace_seconds: float) -> None:
    if process_group_id <= 1 or process_group_id == os.getpgrp():
        raise NativeBuildError(f"refusing to signal unsafe process group {process_group_id}")
    if _group_exists(process_group_id):
        try:
            os.killpg(process_group_id, signal.SIGTERM)
        except (ProcessLookupError, PermissionError):
            pass
    deadline = time.monotonic() + grace_seconds
    while time.monotonic() < deadline:
        if not _group_exists(process_group_id):
            break
        time.sleep(0.05)
    if _group_exists(process_group_id):
        try:
            os.killpg(process_group_id, signal.SIGKILL)
        except (ProcessLookupError, PermissionError):
            pass
    kill_deadline = time.monotonic() + max(1.0, grace_seconds)
    while time.monotonic() < kill_deadline:
        if not _group_exists(process_group_id):
            break
        time.sleep(0.05)
    if _group_exists(process_group_id):
        raise NativeBuildError(
            f"process group {process_group_id} не исчезла после SIGKILL"
        )


def guard_state_path(log_path: Path) -> Path:
    return log_path.with_name(f".{log_path.name}.guard-state.json")


def _read_watchdog_events(
    control: socket.socket, buffer: bytes, timeout_seconds: float
) -> tuple[list[dict[str, Any]], bytes, bool]:
    control.settimeout(timeout_seconds)
    try:
        chunk = control.recv(4096)
    except (BlockingIOError, TimeoutError, socket.timeout):
        return [], buffer, False
    if not chunk:
        return [], buffer, True
    buffer += chunk
    if len(buffer) > 16 * 1024:
        raise NativeBuildError("watchdog event stream exceeded hard cap")
    events: list[dict[str, Any]] = []
    while b"\n" in buffer:
        line, buffer = buffer.split(b"\n", 1)
        try:
            event = json.loads(line.decode("ascii"))
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise NativeBuildError("watchdog returned invalid event JSON") from error
        if not isinstance(event, dict) or not isinstance(event.get("event"), str):
            raise NativeBuildError("watchdog returned invalid event shape")
        events.append(event)
    return events, buffer, False


def _terminate_watchdog(
    watchdog: subprocess.Popen[bytes], grace_seconds: float
) -> None:
    if watchdog.poll() is not None:
        return
    try:
        os.killpg(watchdog.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    try:
        watchdog.wait(timeout=max(2.0, grace_seconds * 3))
        return
    except subprocess.TimeoutExpired:
        pass
    try:
        os.killpg(watchdog.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    watchdog.wait(timeout=max(1.0, grace_seconds))


def run_guarded(
    command: Sequence[str],
    *,
    cwd: Path,
    environment: Mapping[str, str],
    log_path: Path,
    timeout_seconds: float,
    maximum_tree_bytes: int,
    maximum_group_rss_bytes: int,
    maximum_log_bytes: int,
    maximum_group_processes: int = 64,
    termination_grace_seconds: float = 1.0,
    poll_seconds: float = 0.5,
    work_id: str | None = None,
    keepalive_fds: Sequence[int] = (),
) -> GuardMetrics:
    """Запускает target через crash-safe watchdog и применяет sampled caps."""
    if not command or any(not isinstance(argument, str) for argument in command):
        raise ValueError("command must be a non-empty sequence of strings")
    numeric_limits = (
        timeout_seconds,
        maximum_tree_bytes,
        maximum_group_rss_bytes,
        maximum_log_bytes,
        maximum_group_processes,
        termination_grace_seconds,
        poll_seconds,
    )
    if any(isinstance(value, bool) or value <= 0 for value in numeric_limits):
        raise ValueError("all process guard limits must be positive")
    if work_id is None:
        work_id = hashlib.sha256(
            f"{os.getpid()}:{time.monotonic_ns()}".encode("ascii")
        ).hexdigest()[: WORK_ID_HEX_BYTES * 2]
    if not _valid_work_id(work_id):
        raise ValueError("guard work_id must be 32 lowercase hexadecimal characters")
    normalized_keepalive_fds: list[int] = []
    for descriptor in keepalive_fds:
        if (
            isinstance(descriptor, bool)
            or not isinstance(descriptor, int)
            or descriptor <= 2
            or descriptor in normalized_keepalive_fds
        ):
            raise ValueError("guard keepalive descriptors must be unique and > 2")
        try:
            os.fstat(descriptor)
        except OSError as error:
            raise ValueError(f"guard keepalive descriptor is closed: {descriptor}") from error
        normalized_keepalive_fds.append(descriptor)
    cwd = cwd.resolve(strict=True)
    if not cwd.is_dir():
        raise ValueError(f"guard cwd is not a directory: {cwd}")
    log_path.parent.mkdir(parents=True, exist_ok=True)
    if log_path.exists() and not stat.S_ISREG(log_path.lstat().st_mode):
        raise ValueError(f"guard log is not a regular file: {log_path}")
    state_path = guard_state_path(log_path)
    if state_path.exists() or state_path.is_symlink():
        raise ValueError(f"stale watchdog state exists: {state_path}")

    log_flags = os.O_CREAT | os.O_TRUNC | os.O_WRONLY
    if hasattr(os, "O_NOFOLLOW"):
        log_flags |= os.O_NOFOLLOW
    log_descriptor = os.open(log_path, log_flags, 0o600)
    watchdog: subprocess.Popen[bytes] | None = None
    parent_control: socket.socket | None = None
    child_control: socket.socket | None = None
    process_group_id = -1
    started = time.monotonic()
    peak_tree = 0
    peak_rss = 0
    peak_processes = 0
    violation: str | None = None
    return_code: int | None = None
    next_tree_scan = started
    event_buffer = b""
    try:
        parent_control, child_control = socket.socketpair()
        watchdog = subprocess.Popen(
            [
                sys.executable,
                str(ROOT / "scripts/process-group-watchdog.py"),
                "--control-fd",
                str(child_control.fileno()),
                "--state-path",
                str(state_path),
                "--guard-parent-pid",
                str(os.getpid()),
                "--work-id",
                work_id,
                "--grace-seconds",
                str(termination_grace_seconds),
                "--cwd",
                str(cwd),
                "--",
                *command,
            ],
            cwd=cwd,
            env=dict(environment),
            stdin=subprocess.DEVNULL,
            stdout=log_descriptor,
            stderr=subprocess.STDOUT,
            close_fds=True,
            start_new_session=True,
            shell=False,
            pass_fds=(child_control.fileno(), *normalized_keepalive_fds),
        )
        child_control.close()
        child_control = None
        handshake_deadline = time.monotonic() + min(10.0, timeout_seconds)
        while process_group_id < 0:
            events, event_buffer, closed = _read_watchdog_events(
                parent_control, event_buffer, 0.1
            )
            for event in events:
                if event["event"] == "started":
                    candidate = event.get("target_process_group_id")
                    if (
                        isinstance(candidate, bool)
                        or not isinstance(candidate, int)
                        or candidate <= 1
                        or candidate == os.getpgrp()
                    ):
                        raise NativeBuildError("watchdog returned unsafe target PGID")
                    process_group_id = candidate
                elif event["event"] == "exited":
                    return_code = int(event["return_code"])
            if closed:
                if process_group_id < 0:
                    detail = " with incomplete event" if event_buffer else ""
                    raise NativeBuildError(
                        "watchdog exited before target handshake" + detail
                    )
                break
            if time.monotonic() > handshake_deadline:
                raise NativeBuildError("watchdog target handshake timed out")

        while True:
            now = time.monotonic()
            events, event_buffer, closed = _read_watchdog_events(
                parent_control, event_buffer, 0.0
            )
            for event in events:
                if event["event"] == "exited":
                    candidate = event.get("return_code")
                    if isinstance(candidate, bool) or not isinstance(candidate, int):
                        raise NativeBuildError("watchdog returned invalid exit status")
                    return_code = candidate
            if closed or watchdog.poll() is not None:
                violation = "watchdog exited before guarded cleanup"
            if now >= next_tree_scan:
                peak_tree = max(peak_tree, directory_tree_bytes(cwd))
                next_tree_scan = now + max(0.1, poll_seconds * 5)
                if peak_tree > maximum_tree_bytes:
                    violation = (
                        f"build tree cap exceeded: {peak_tree} > {maximum_tree_bytes}"
                    )
            rss, processes = _group_usage(process_group_id)
            watchdog_rss, watchdog_processes = _group_usage(watchdog.pid)
            rss += watchdog_rss
            processes += watchdog_processes
            peak_rss = max(peak_rss, rss)
            peak_processes = max(peak_processes, processes)
            if rss > maximum_group_rss_bytes:
                violation = (
                    f"process group RSS cap exceeded: {rss} > "
                    f"{maximum_group_rss_bytes}"
                )
            if processes > maximum_group_processes:
                violation = (
                    f"process group count cap exceeded: {processes} > "
                    f"{maximum_group_processes}"
                )
            log_bytes = os.fstat(log_descriptor).st_size
            if log_bytes > maximum_log_bytes:
                violation = f"log cap exceeded: {log_bytes} > {maximum_log_bytes}"
            if now - started > timeout_seconds:
                violation = f"command timeout exceeded: {timeout_seconds} seconds"
            if violation is not None or return_code is not None:
                break
            time.sleep(poll_seconds)
    except BaseException as error:
        violation = violation or f"process guard failed: {type(error).__name__}: {error}"
    finally:
        cleanup_errors: list[str] = []
        try:
            if parent_control is not None:
                try:
                    parent_control.sendall(b"CLEANUP\n")
                except (BrokenPipeError, ConnectionResetError, OSError) as error:
                    cleanup_errors.append(f"watchdog control failed: {error}")
            if watchdog is not None:
                try:
                    watchdog.wait(
                        timeout=max(5.0, termination_grace_seconds * 4 + 5)
                    )
                except subprocess.TimeoutExpired:
                    cleanup_errors.append("watchdog cleanup timed out")
                    _terminate_watchdog(watchdog, termination_grace_seconds)
                if watchdog.returncode not in (0, None):
                    cleanup_errors.append(
                        f"watchdog exited with status {watchdog.returncode}"
                    )
            if process_group_id > 1 and _group_exists(process_group_id):
                try:
                    _terminate_orphan_group(
                        process_group_id, termination_grace_seconds
                    )
                except BaseException as error:
                    cleanup_errors.append(f"fallback target cleanup failed: {error}")
            if state_path.exists() or state_path.is_symlink():
                if process_group_id > 1 and not _group_exists(process_group_id):
                    state_path.unlink(missing_ok=True)
                else:
                    cleanup_errors.append(f"watchdog state remains: {state_path}")
        except BaseException as cleanup_error:
            cleanup_errors.append(str(cleanup_error))
        finally:
            if child_control is not None:
                child_control.close()
            if parent_control is not None:
                parent_control.close()
            os.close(log_descriptor)
        if cleanup_errors:
            cleanup_message = "; ".join(cleanup_errors)
            violation = (
                f"{violation}; cleanup failed: {cleanup_message}"
                if violation
                else f"cleanup failed: {cleanup_message}"
            )

    duration = time.monotonic() - started
    try:
        final_tree = directory_tree_bytes(cwd)
    except (OSError, NativeBuildError):
        final_tree = peak_tree
    peak_tree = max(peak_tree, final_tree)
    try:
        log_bytes = log_path.lstat().st_size
    except FileNotFoundError:
        log_bytes = 0
    metrics = GuardMetrics(
        process_group_id=process_group_id,
        duration_seconds=duration,
        peak_tree_bytes=peak_tree,
        peak_group_rss_bytes=peak_rss,
        peak_group_processes=peak_processes,
        log_bytes=log_bytes,
        return_code=return_code if return_code is not None else -1,
    )
    if violation is not None:
        raise BuildGuardError(violation, process_group_id, metrics)
    if return_code != 0:
        raise BuildGuardError(
            f"command exited with status {return_code}", process_group_id, metrics
        )
    if peak_tree > maximum_tree_bytes or log_bytes > maximum_log_bytes:
        raise BuildGuardError(
            "resource cap exceeded between final polls", process_group_id, metrics
        )
    return metrics


def safe_extract(
    archive_path: Path,
    destination: Path,
    *,
    maximum_bytes: int,
    expected_root: str,
    maximum_members: int = 50_000,
    maximum_member_bytes: int = 16 * 1024 * 1024,
    maximum_path_bytes: int = 512,
    maximum_path_depth: int = 32,
) -> tuple[Path, int, int]:
    """Извлекает только regular files/directories из единственного tar root."""
    if destination.exists() or destination.is_symlink():
        raise ValueError(f"extract destination must not exist: {destination}")
    if not expected_root or "/" in expected_root or expected_root in {".", ".."}:
        raise ValueError("expected tar root is unsafe")
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.mkdir(mode=0o700)
    extracted_bytes = 0
    members_count = 0
    seen: set[str] = set()
    try:
        with tarfile.open(archive_path, "r:gz") as archive:
            members: list[tarfile.TarInfo] = []
            for member in archive:
                members.append(member)
                if len(members) > maximum_members:
                    raise ValueError(
                        f"tar member count exceeds hard cap {maximum_members}"
                    )
                members_count += 1
                name = member.name
                if (
                    not name
                    or "\\" in name
                    or "\x00" in name
                    or len(name.encode("utf-8")) > maximum_path_bytes
                ):
                    raise ValueError(f"unsafe tar path: {name!r}")
                path = PurePosixPath(name)
                parts = path.parts
                if (
                    path.is_absolute()
                    or not parts
                    or parts[0] != expected_root
                    or any(part in {"", ".", ".."} for part in parts)
                    or len(parts) > maximum_path_depth
                ):
                    raise ValueError(f"tar path escapes exact root: {name!r}")
                if name in seen:
                    raise ValueError(f"duplicate tar member: {name}")
                seen.add(name)
                if not (member.isdir() or member.isreg()):
                    raise ValueError(f"unsupported tar member type: {name}")
                if member.size < 0 or member.size > maximum_member_bytes:
                    raise ValueError(f"tar member exceeds hard cap: {name}")
                extracted_bytes += member.size
                if extracted_bytes > maximum_bytes:
                    raise ValueError(
                        f"tar logical size exceeds hard cap {maximum_bytes}"
                    )

            for member in members:
                output_path = destination.joinpath(*PurePosixPath(member.name).parts)
                if member.isdir():
                    output_path.mkdir(parents=True, exist_ok=True, mode=0o755)
                    output_path.chmod(0o755)
                    continue
                output_path.parent.mkdir(parents=True, exist_ok=True, mode=0o755)
                source = archive.extractfile(member)
                if source is None:
                    raise ValueError(f"tar regular member has no payload: {member.name}")
                flags = os.O_CREAT | os.O_EXCL | os.O_WRONLY
                if hasattr(os, "O_NOFOLLOW"):
                    flags |= os.O_NOFOLLOW
                mode = 0o755 if member.mode & 0o111 else 0o644
                descriptor = os.open(output_path, flags, mode)
                written = 0
                try:
                    with os.fdopen(descriptor, "wb", closefd=False) as output:
                        while True:
                            chunk = source.read(1024 * 1024)
                            if not chunk:
                                break
                            written += len(chunk)
                            if written > member.size:
                                raise ValueError(
                                    "tar member payload exceeds declared size: "
                                    f"{member.name}"
                                )
                            output.write(chunk)
                finally:
                    os.close(descriptor)
                    source.close()
                if written != member.size:
                    raise ValueError(
                        f"tar member payload size mismatch: {member.name}"
                    )
        actual_tree_bytes = directory_tree_bytes(destination)
        if actual_tree_bytes > maximum_bytes + 16 * 1024 * 1024:
            raise ValueError("extracted tree exceeds logical source cap")
        source_root = destination / expected_root
        if not source_root.is_dir() or source_root.is_symlink():
            raise ValueError("exact tar root was not extracted as directory")
        return source_root, extracted_bytes, members_count
    except BaseException:
        shutil.rmtree(destination, ignore_errors=True)
        raise


def minimal_environment(work_root: Path, sdk_path: Path) -> dict[str, str]:
    home = work_root / "home"
    temporary = work_root / "tmp"
    home.mkdir(mode=0o700)
    temporary.mkdir(mode=0o700)
    return {
        "PATH": "/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin",
        "HOME": str(home),
        "TMPDIR": f"{temporary}/",
        "SDKROOT": str(sdk_path),
        "CC": "/usr/bin/clang",
        "CXX": "/usr/bin/clang++",
        "CMAKE_BUILD_PARALLEL_LEVEL": "2",
        "MAKEFLAGS": "-j2",
        "LC_ALL": "C",
        "LANG": "C",
        "TZ": "UTC",
        "ZERO_AR_DATE": "1",
        "PYTHONDONTWRITEBYTECODE": "1",
        "CCACHE_DISABLE": "1",
        "SCCACHE_DISABLE": "1",
    }


def _tool_path(name: str) -> str:
    path = shutil.which(name, path="/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin")
    if path is None:
        raise NativeBuildError(f"required build tool is missing: {name}")
    return str(Path(path).resolve())


def capture_command(
    command: Sequence[str],
    *,
    timeout_seconds: int,
    maximum_rss_bytes: int,
    maximum_processes: int = 4,
    scratch_root: Path | None = None,
    work_id: str | None = None,
    keepalive_fds: Sequence[int] = (),
) -> str:
    temporary_root = scratch_root or NATIVE_ROOT
    temporary_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".inspect-", dir=temporary_root) as directory:
        work = Path(directory)
        log = work / "output.log"
        run_guarded(
            command,
            cwd=work,
            environment={
                "PATH": "/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin",
                "LC_ALL": "C",
                "LANG": "C",
                "PYTHONDONTWRITEBYTECODE": "1",
            },
            log_path=log,
            timeout_seconds=timeout_seconds,
            maximum_tree_bytes=4 * 1024 * 1024,
            maximum_group_rss_bytes=maximum_rss_bytes,
            maximum_group_processes=maximum_processes,
            maximum_log_bytes=MAX_INSPECTION_OUTPUT_BYTES,
            termination_grace_seconds=1,
            poll_seconds=0.05,
            work_id=work_id,
            keepalive_fds=keepalive_fds,
        )
        return read_bounded(
            log, MAX_INSPECTION_OUTPUT_BYTES, "inspection command output"
        ).decode("utf-8", errors="strict").strip()


def smoke_artifact(
    artifact: Path,
    policy: dict[str, Any],
    *,
    scratch_root: Path | None = None,
    work_id: str | None = None,
    keepalive_fds: Sequence[int] = (),
) -> dict[str, Any]:
    limits = policy["limits"]
    temporary_root = scratch_root or NATIVE_ROOT
    temporary_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".smoke-", dir=temporary_root) as directory:
        work = Path(directory)
        output = work / "result.json"
        log = work / "smoke.log"
        run_guarded(
            [
                sys.executable,
                str(ROOT / "scripts/smoke-tdlib-native.py"),
                "--artifact",
                str(artifact),
                "--expected-version",
                policy["source"]["version"],
                "--expected-commit",
                policy["source"]["commit"],
                "--output",
                str(output),
            ],
            cwd=work,
            environment={
                "PATH": "/usr/bin:/bin",
                "LC_ALL": "C",
                "LANG": "C",
                "PYTHONDONTWRITEBYTECODE": "1",
            },
            log_path=log,
            timeout_seconds=limits["inspection_seconds"],
            maximum_tree_bytes=2 * 1024 * 1024,
            maximum_group_rss_bytes=limits["inspection_rss_bytes"],
            maximum_group_processes=4,
            maximum_log_bytes=1024 * 1024,
            termination_grace_seconds=1,
            poll_seconds=0.05,
            work_id=work_id,
            keepalive_fds=keepalive_fds,
        )
        result = read_json_bounded(output, 32 * 1024, "TDJSON isolated smoke")
    expected = {
        "format_version": 1,
        "options": {
            "version": policy["source"]["version"],
            "commit_hash": policy["source"]["commit"],
        },
        "database_files_created": 0,
    }
    errors = exact_contract_errors(result, expected, "smoke")
    if errors:
        raise NativeBuildError("; ".join(errors))
    return result


def inspect_artifact(
    artifact: Path,
    policy: dict[str, Any],
    *,
    scratch_root: Path | None = None,
    work_id: str | None = None,
    keepalive_fds: Sequence[int] = (),
) -> dict[str, Any]:
    limits = policy["limits"]
    digest, artifact_bytes = sha256_file(
        artifact, limits["artifact_bytes"], "native artifact"
    )
    timeout = limits["inspection_seconds"]
    inspection_rss = limits["inspection_rss_bytes"]
    guard_arguments = {
        "scratch_root": scratch_root,
        "work_id": work_id,
        "keepalive_fds": keepalive_fds,
    }

    file_format = capture_command(
        [_tool_path("file"), "-b", str(artifact)],
        timeout_seconds=timeout,
        maximum_rss_bytes=inspection_rss,
        **guard_arguments,
    )
    if "Mach-O 64-bit dynamically linked shared library arm64" not in file_format:
        raise NativeBuildError(f"artifact is not an arm64 Mach-O dylib: {file_format}")

    architectures = capture_command(
        [_tool_path("lipo"), "-archs", str(artifact)],
        timeout_seconds=timeout,
        maximum_rss_bytes=inspection_rss,
        **guard_arguments,
    ).split()
    if architectures != ["arm64"]:
        raise NativeBuildError(f"artifact architectures differ: {architectures}")

    install_output = capture_command(
        [_tool_path("otool"), "-D", str(artifact)],
        timeout_seconds=timeout,
        maximum_rss_bytes=inspection_rss,
        **guard_arguments,
    ).splitlines()
    if len(install_output) != 2:
        raise NativeBuildError("artifact has ambiguous Mach-O install name")
    install_name = install_output[1].strip()
    expected_install_name = f"@rpath/{policy['target']['artifact_basename']}"
    if install_name != expected_install_name:
        raise NativeBuildError(
            f"artifact install name differs: {install_name!r} != "
            f"{expected_install_name!r}"
        )

    linked_output = capture_command(
        [_tool_path("otool"), "-L", str(artifact)],
        timeout_seconds=timeout,
        maximum_rss_bytes=inspection_rss,
        **guard_arguments,
    ).splitlines()
    dependencies: list[str] = []
    for line in linked_output[1:]:
        dependency = line.strip().split(" (compatibility version", 1)[0]
        if dependency == install_name:
            continue
        if not dependency.startswith(("/usr/lib/", "/System/Library/")):
            raise NativeBuildError(
                f"artifact contains non-system dynamic dependency: {dependency}"
            )
        dependencies.append(dependency)
    if not dependencies:
        raise NativeBuildError("artifact dynamic dependency inventory is empty")

    load_commands = capture_command(
        [_tool_path("otool"), "-l", str(artifact)],
        timeout_seconds=timeout,
        maximum_rss_bytes=inspection_rss,
        **guard_arguments,
    )
    lines = [line.strip() for line in load_commands.splitlines()]
    rpaths = [
        lines[index + 2].split(" (offset", 1)[0].removeprefix("path ")
        for index, line in enumerate(lines[:-2])
        if line == "cmd LC_RPATH" and lines[index + 2].startswith("path ")
    ]
    if rpaths:
        raise NativeBuildError(f"artifact contains LC_RPATH entries: {rpaths}")
    minimum_macos = _minimum_macos_version(lines)
    if minimum_macos != "11.0":
        raise NativeBuildError(
            f"artifact minimum macOS differs from exact policy: {minimum_macos}"
        )

    symbols = capture_command(
        [_tool_path("nm"), "-gUj", str(artifact)],
        timeout_seconds=timeout,
        maximum_rss_bytes=inspection_rss,
        **guard_arguments,
    ).splitlines()
    normalized_symbols = {symbol.strip().removeprefix("_") for symbol in symbols}
    missing_exports = sorted(set(REQUIRED_EXPORTS) - normalized_symbols)
    if missing_exports:
        raise NativeBuildError(
            "artifact misses required TDJSON exports: " + ", ".join(missing_exports)
        )

    smoke = smoke_artifact(artifact, policy, **guard_arguments)
    return {
        "sha256": digest,
        "bytes": artifact_bytes,
        "verification": {
            "file_format": file_format,
            "architectures": architectures,
            "minimum_macos": minimum_macos,
            "install_name": install_name,
            "rpaths": rpaths,
            "dynamic_dependencies": dependencies,
            "exports": list(REQUIRED_EXPORTS),
            "options": smoke["options"],
            "database_files_created": smoke["database_files_created"],
        },
    }


def _minimum_macos_version(load_command_lines: Sequence[str]) -> str:
    for index, line in enumerate(load_command_lines[:-1]):
        if line == "cmd LC_BUILD_VERSION":
            for candidate in load_command_lines[index + 1 : index + 8]:
                if candidate.startswith("minos "):
                    return candidate.split(maxsplit=1)[1]
        if line == "cmd LC_VERSION_MIN_MACOSX":
            for candidate in load_command_lines[index + 1 : index + 6]:
                if candidate.startswith("version "):
                    return candidate.split(maxsplit=1)[1]
    raise NativeBuildError("artifact has no macOS minimum-version load command")


def expected_target_record(policy: dict[str, Any]) -> dict[str, Any]:
    return {
        "triple": policy["target"]["triple"],
        "cmake_generator": policy["target"]["cmake_generator"],
        "cmake_target": policy["target"]["cmake_target"],
        "parallel_jobs": policy["limits"]["parallel_jobs"],
        "cmake_defines": policy["target"]["cmake_defines"],
    }


def provenance_errors(
    provenance: dict[str, Any], policy: dict[str, Any]
) -> list[str]:
    expected_top = {
        "format_version",
        "source",
        "policy_sha256",
        "recipe",
        "target",
        "build",
        "artifact",
        "verification",
        "reproducibility",
    }
    errors: list[str] = []
    if set(provenance) != expected_top:
        errors.append(
            f"provenance top-level fields differ: {sorted(set(provenance) ^ expected_top)}"
        )
    if provenance.get("format_version") != 1:
        errors.append("provenance.format_version must be 1")
    if provenance.get("source") != policy["source"]:
        errors.append("provenance.source differs from exact policy")
    if provenance.get("policy_sha256") != canonical_sha256(policy):
        errors.append("provenance.policy_sha256 differs from exact policy")
    expected_recipe = {"files": recipe_fingerprints()}
    if provenance.get("recipe") != expected_recipe:
        errors.append("provenance.recipe differs from current bounded build recipe")
    if provenance.get("target") != expected_target_record(policy):
        errors.append("provenance.target differs from exact target contract")

    build = provenance.get("build")
    if not isinstance(build, dict):
        errors.append("provenance.build must be an object")
    else:
        errors.extend(_build_record_errors(build, policy))

    artifact = provenance.get("artifact")
    if not isinstance(artifact, dict) or set(artifact) != {
        "cache_path",
        "sha256",
        "bytes",
    }:
        errors.append("provenance.artifact has invalid closed schema")
    else:
        digest = artifact.get("sha256")
        if (
            not isinstance(digest, str)
            or len(digest) != 64
            or any(character not in "0123456789abcdef" for character in digest)
        ):
            errors.append("provenance artifact sha256 is invalid")
        elif artifact.get("cache_path") != str(
            Path(policy["target"]["artifact_cache_directory"])
            / digest
            / policy["target"]["artifact_basename"]
        ):
            errors.append("provenance artifact cache path differs")
        artifact_bytes = artifact.get("bytes")
        if (
            isinstance(artifact_bytes, bool)
            or not isinstance(artifact_bytes, int)
            or not 0 < artifact_bytes <= policy["limits"]["artifact_bytes"]
        ):
            errors.append("provenance artifact bytes exceed exact cap")

    verification = provenance.get("verification")
    errors.extend(_verification_record_errors(verification, policy))
    expected_reproducibility = {
        "status": "verified",
        "independent_builds": 2,
        "claim": "independent exact-recipe builds are bit-for-bit identical",
    }
    if provenance.get("reproducibility") != expected_reproducibility:
        errors.append("provenance.reproducibility overclaims or differs")
    return errors


def _build_record_errors(build: dict[str, Any], policy: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    expected_keys = {
        "built_at_utc",
        "host",
        "toolchain",
        "dependencies",
        "source_preparation",
        "phases",
    }
    if set(build) != expected_keys:
        errors.append("provenance.build fields differ from closed schema")
        return errors
    timestamp = build.get("built_at_utc")
    try:
        parsed = datetime.fromisoformat(str(timestamp).replace("Z", "+00:00"))
        if parsed.tzinfo is None:
            raise ValueError("timezone is absent")
    except ValueError:
        errors.append("provenance.build.built_at_utc is not timezone-aware ISO-8601")

    host = build.get("host")
    if not isinstance(host, dict) or set(host) != {"system", "release", "machine"}:
        errors.append("provenance.build.host has invalid closed schema")
    elif host.get("system") != "Darwin" or host.get("machine") != "arm64":
        errors.append("provenance build host is not native macOS arm64")

    toolchain = build.get("toolchain")
    expected_tools = {"cmake", "make", "clang", "gperf", "python", "xcode_sdk"}
    if not isinstance(toolchain, dict) or set(toolchain) != expected_tools:
        errors.append("provenance.build.toolchain has invalid closed schema")
    elif any(
        not isinstance(value, str) or not value or len(value) > 1024
        for value in toolchain.values()
    ):
        errors.append("provenance.build.toolchain contains invalid value")
    elif any(
        marker in value
        for value in toolchain.values()
        for marker in ("/Users/", "/private/tmp/", "/var/folders/", ".work-")
    ):
        errors.append("provenance.build.toolchain contains private/scratch path")

    dependencies = build.get("dependencies")
    if not isinstance(dependencies, dict) or set(dependencies) != {"openssl", "zlib"}:
        errors.append("provenance.build.dependencies has invalid closed schema")
    else:
        openssl = dependencies.get("openssl")
        if not isinstance(openssl, dict) or set(openssl) != {
            "formula",
            "version",
            "prefix",
            "cellar_path",
            "linkage",
            "libssl_sha256",
            "libssl_bytes",
            "libcrypto_sha256",
            "libcrypto_bytes",
        }:
            errors.append("provenance OpenSSL dependency has invalid closed schema")
        elif (
            openssl.get("formula") != "openssl@3"
            or openssl.get("prefix") != "/opt/homebrew/opt/openssl@3"
            or openssl.get("linkage") != "static"
        ):
            errors.append("provenance OpenSSL resolution differs from policy")
        elif (
            not isinstance(openssl.get("cellar_path"), str)
            or not openssl["cellar_path"].startswith(
                "/opt/homebrew/Cellar/openssl@3/"
            )
            or "/../" in openssl["cellar_path"]
        ):
            errors.append("provenance OpenSSL Cellar path is invalid")
        elif (
            not isinstance(openssl.get("version"), str)
            or not openssl["version"]
            or len(openssl["version"]) > 512
        ):
            errors.append("provenance OpenSSL version is invalid")
        else:
            for name in ("libssl", "libcrypto"):
                digest = openssl.get(f"{name}_sha256")
                size = openssl.get(f"{name}_bytes")
                if (
                    not isinstance(digest, str)
                    or len(digest) != 64
                    or any(character not in "0123456789abcdef" for character in digest)
                ):
                    errors.append(f"provenance {name} hash is invalid")
                if (
                    isinstance(size, bool)
                    or not isinstance(size, int)
                    or not 0 < size <= 16 * 1024 * 1024
                ):
                    errors.append(f"provenance {name} bytes exceed cap")
        zlib = dependencies.get("zlib")
        if zlib != {"provider": "macos-sdk", "linkage": "system"}:
            errors.append("provenance zlib resolution differs from policy")

    source_preparation = build.get("source_preparation")
    if not isinstance(source_preparation, dict) or set(source_preparation) != {
        "archive_sha256",
        "archive_bytes",
        "archive_cache_reused",
        "extracted_logical_bytes",
        "extracted_members",
        "commit_identity",
    }:
        errors.append("provenance source preparation has invalid closed schema")
    else:
        source = policy["source"]
        if source_preparation.get("archive_sha256") != source["archive_sha256"]:
            errors.append("provenance source archive hash differs")
        if source_preparation.get("archive_bytes") != source["archive_bytes"]:
            errors.append("provenance source archive bytes differ")
        if not isinstance(source_preparation.get("archive_cache_reused"), bool):
            errors.append("provenance archive cache reuse marker is invalid")
        extracted = source_preparation.get("extracted_logical_bytes")
        members = source_preparation.get("extracted_members")
        if (
            isinstance(extracted, bool)
            or not isinstance(extracted, int)
            or not 0 < extracted <= policy["limits"]["extracted_source_bytes"]
        ):
            errors.append("provenance extracted source bytes exceed cap")
        if (
            isinstance(members, bool)
            or not isinstance(members, int)
            or not 0 < members <= policy["limits"]["source_archive_members"]
        ):
            errors.append("provenance extracted member count exceeds cap")
        expected_commit_identity = {
            "strategy": policy["source"]["commit_identity_strategy"],
            "commit": policy["source"]["commit"],
            "head_sha256": policy["source"]["git_head_sha256"],
            "template_sha256": policy["source"]["git_commit_template_sha256"],
            "generated_sha256": policy["source"]["git_commit_generated_sha256"],
        }
        if source_preparation.get("commit_identity") != expected_commit_identity:
            errors.append("provenance commit identity differs from exact policy")

    phases = build.get("phases")
    if not isinstance(phases, dict) or set(phases) != {"configure", "build"}:
        errors.append("provenance build phases have invalid closed schema")
    else:
        for name, timeout_field in (
            ("configure", "configure_seconds"),
            ("build", "build_seconds"),
        ):
            errors.extend(
                _phase_record_errors(phases.get(name), name, timeout_field, policy)
            )
    return errors


def _phase_record_errors(
    phase: Any, name: str, timeout_field: str, policy: dict[str, Any]
) -> list[str]:
    expected = {
        "duration_seconds",
        "peak_tree_bytes",
        "peak_group_rss_bytes",
        "peak_group_processes",
        "log_bytes",
        "log_sha256",
        "return_code",
    }
    if not isinstance(phase, dict) or set(phase) != expected:
        return [f"provenance phase {name} has invalid closed schema"]
    errors: list[str] = []
    limits = policy["limits"]
    numeric_caps = {
        "duration_seconds": limits[timeout_field] + 2,
        "peak_tree_bytes": limits["build_tree_bytes"],
        "peak_group_rss_bytes": limits["process_group_rss_bytes"],
        "peak_group_processes": limits["process_group_processes"],
        "log_bytes": limits["log_bytes"],
    }
    for field, maximum in numeric_caps.items():
        value = phase.get(field)
        if (
            isinstance(value, bool)
            or not isinstance(value, (int, float))
            or value < 0
            or value > maximum
        ):
            errors.append(f"provenance phase {name}.{field} exceeds cap")
    digest = phase.get("log_sha256")
    if (
        not isinstance(digest, str)
        or len(digest) != 64
        or any(character not in "0123456789abcdef" for character in digest)
    ):
        errors.append(f"provenance phase {name}.log_sha256 is invalid")
    if phase.get("return_code") != 0:
        errors.append(f"provenance phase {name} did not exit successfully")
    return errors


def _verification_record_errors(
    verification: Any, policy: dict[str, Any]
) -> list[str]:
    expected_keys = {
        "file_format",
        "architectures",
        "minimum_macos",
        "install_name",
        "rpaths",
        "dynamic_dependencies",
        "exports",
        "options",
        "database_files_created",
    }
    if not isinstance(verification, dict) or set(verification) != expected_keys:
        return ["provenance.verification has invalid closed schema"]
    errors: list[str] = []
    if "Mach-O 64-bit dynamically linked shared library arm64" not in str(
        verification.get("file_format", "")
    ):
        errors.append("provenance artifact format is not arm64 Mach-O dylib")
    if verification.get("architectures") != ["arm64"]:
        errors.append("provenance artifact architecture differs")
    if verification.get("minimum_macos") != "11.0":
        errors.append("provenance artifact deployment target differs")
    if verification.get("install_name") != (
        f"@rpath/{policy['target']['artifact_basename']}"
    ):
        errors.append("provenance artifact install name differs")
    if verification.get("rpaths") != []:
        errors.append("provenance artifact contains rpaths")
    dependencies = verification.get("dynamic_dependencies")
    if not isinstance(dependencies, list) or not dependencies:
        errors.append("provenance dynamic dependency inventory is empty")
    elif any(
        not isinstance(item, str)
        or not item.startswith(("/usr/lib/", "/System/Library/"))
        for item in dependencies
    ):
        errors.append("provenance contains non-system dynamic dependency")
    elif len(set(dependencies)) != len(dependencies):
        errors.append("provenance dynamic dependency inventory has duplicates")
    if verification.get("exports") != list(REQUIRED_EXPORTS):
        errors.append("provenance required exports differ")
    expected_options = {
        "version": policy["source"]["version"],
        "commit_hash": policy["source"]["commit"],
    }
    if verification.get("options") != expected_options:
        errors.append("provenance TDJSON version/commit smoke differs")
    if verification.get("database_files_created") != 0:
        errors.append("provenance smoke created database files")
    return errors


def local_artifact_errors(
    provenance: dict[str, Any], inspection: dict[str, Any]
) -> list[str]:
    errors: list[str] = []
    artifact = provenance.get("artifact", {})
    if artifact.get("sha256") != inspection.get("sha256"):
        errors.append("local artifact sha256 differs from committed provenance")
    if artifact.get("bytes") != inspection.get("bytes"):
        errors.append("local artifact bytes differ from committed provenance")
    if provenance.get("verification") != inspection.get("verification"):
        errors.append("local artifact verification differs from committed provenance")
    return errors


def artifact_cache_path(policy: dict[str, Any], digest: str) -> Path:
    if len(digest) != 64 or any(
        character not in "0123456789abcdef" for character in digest
    ):
        raise NativeBuildError("native artifact digest is invalid")
    relative = (
        Path(policy["target"]["artifact_cache_directory"])
        / digest
        / policy["target"]["artifact_basename"]
    )
    resolved = (ROOT / relative).resolve()
    native_root = NATIVE_ROOT.resolve()
    if resolved == native_root or native_root not in resolved.parents:
        raise NativeBuildError("native artifact cache path escapes target root")
    return resolved


def clone_policy_with(policy: dict[str, Any], path: Sequence[str], value: Any) -> dict[str, Any]:
    candidate = copy.deepcopy(policy)
    cursor: dict[str, Any] = candidate
    for field in path[:-1]:
        cursor = cursor[field]
    cursor[path[-1]] = value
    return candidate
