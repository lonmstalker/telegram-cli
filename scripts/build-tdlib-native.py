#!/usr/bin/env python3
"""Собирает exact TDLib tdjson в bounded private scratch и публикует provenance."""

from __future__ import annotations

import argparse
from contextlib import contextmanager
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import signal
import stat
import sys
import tempfile
import time
from typing import Any, Iterator
from urllib.parse import urlparse
import urllib.request

from tdlib_native import (
    BuildLease,
    BuildGuardError,
    NativeBuildError,
    NATIVE_ROOT,
    PROVENANCE_PATH,
    ROOT,
    artifact_cache_path,
    atomic_copy,
    atomic_write,
    canonical_sha256,
    capture_command,
    cleanup_stale_work_directories,
    exclusive_build_lock,
    expected_target_record,
    guard_state_path,
    inspect_artifact,
    load_exact_contracts,
    local_artifact_errors,
    minimal_environment,
    owned_work_directory,
    provenance_errors,
    read_bounded,
    read_json_bounded,
    recipe_fingerprints,
    run_guarded,
    safe_extract,
    sha256_file,
    _tool_path,
)


ALLOWED_ARCHIVE_HOSTS = {"github.com", "codeload.github.com"}


class RestrictedRedirectHandler(urllib.request.HTTPRedirectHandler):
    """Разрешает redirect только между official GitHub archive endpoints."""

    def redirect_request(self, request: Any, fp: Any, code: int, msg: str, headers: Any, newurl: str) -> Any:
        parsed = urlparse(newurl)
        if parsed.scheme != "https" or parsed.hostname not in ALLOWED_ARCHIVE_HOSTS:
            raise NativeBuildError(f"archive redirect host is not allowed: {parsed.hostname}")
        return super().redirect_request(request, fp, code, msg, headers, newurl)


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--force", action="store_true", help="пересобрать даже валидный local artifact"
    )
    parser.add_argument(
        "--offline",
        action="store_true",
        help="не обращаться к сети; exact archive обязан быть в cache",
    )
    return parser.parse_args()


@contextmanager
def termination_signals_as_errors() -> Iterator[None]:
    previous: dict[int, Any] = {}

    def interrupt(signum: int, _frame: Any) -> None:
        raise NativeBuildError(f"native build interrupted by signal {signum}")

    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        previous[signum] = signal.getsignal(signum)
        signal.signal(signum, interrupt)
    try:
        yield
    finally:
        for signum, handler in previous.items():
            signal.signal(signum, handler)


def _verify_cached_archive(path: Path, policy: dict[str, Any]) -> bool:
    if not path.exists() or path.is_symlink():
        return False
    try:
        digest, size = sha256_file(
            path, policy["limits"]["source_archive_bytes"], "source archive cache"
        )
    except (OSError, ValueError):
        return False
    return (digest, size) == (
        policy["source"]["archive_sha256"],
        policy["source"]["archive_bytes"],
    )


def stage_verified_file(
    source: Path,
    destination: Path,
    *,
    expected_sha256: str,
    expected_bytes: int,
    maximum_bytes: int,
) -> tuple[str, int]:
    """Копирует exact input из одного O_NOFOLLOW fd в private snapshot."""
    if (
        len(expected_sha256) != 64
        or any(character not in "0123456789abcdef" for character in expected_sha256)
        or isinstance(expected_bytes, bool)
        or not 0 < expected_bytes <= maximum_bytes
    ):
        raise NativeBuildError("verified input contract is invalid")
    metadata = source.lstat()
    if not stat.S_ISREG(metadata.st_mode):
        raise NativeBuildError("verified input must be a regular file without symlink")
    source_flags = os.O_RDONLY
    if hasattr(os, "O_NOFOLLOW"):
        source_flags |= os.O_NOFOLLOW
    source_descriptor = os.open(source, source_flags)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination_descriptor = -1
    destination_created = False
    digest = hashlib.sha256()
    copied = 0
    try:
        opened = os.fstat(source_descriptor)
        if not stat.S_ISREG(opened.st_mode) or opened.st_ino != metadata.st_ino:
            raise NativeBuildError("verified input changed before snapshot open")
        destination_flags = os.O_CREAT | os.O_EXCL | os.O_WRONLY
        if hasattr(os, "O_NOFOLLOW"):
            destination_flags |= os.O_NOFOLLOW
        destination_descriptor = os.open(destination, destination_flags, 0o600)
        destination_created = True
        while True:
            chunk = os.read(source_descriptor, 1024 * 1024)
            if not chunk:
                break
            copied += len(chunk)
            if copied > maximum_bytes:
                raise NativeBuildError("verified input snapshot exceeded hard cap")
            digest.update(chunk)
            view = memoryview(chunk)
            while view:
                written = os.write(destination_descriptor, view)
                view = view[written:]
        after = os.fstat(source_descriptor)
        if opened.st_size != after.st_size or opened.st_ino != after.st_ino:
            raise NativeBuildError("verified input changed during snapshot")
        if (digest.hexdigest(), copied) != (expected_sha256, expected_bytes):
            raise NativeBuildError("verified input snapshot differs from exact contract")
        os.fsync(destination_descriptor)
        os.fchmod(destination_descriptor, 0o400)
    except BaseException:
        if destination_created:
            destination.unlink(missing_ok=True)
        raise
    finally:
        os.close(source_descriptor)
        if destination_descriptor >= 0:
            os.close(destination_descriptor)
    return digest.hexdigest(), copied


def download_source_archive(
    policy: dict[str, Any], *, offline: bool
) -> tuple[Path, bool]:
    source = policy["source"]
    limits = policy["limits"]
    archive_directory = NATIVE_ROOT / "downloads"
    archive_directory.mkdir(parents=True, exist_ok=True)
    archive_path = archive_directory / f"{source['archive_sha256']}.tar.gz"
    if _verify_cached_archive(archive_path, policy):
        return archive_path, True
    if offline:
        raise NativeBuildError("offline mode: exact TDLib source archive is absent")

    parsed = urlparse(source["archive_url"])
    if parsed.scheme != "https" or parsed.hostname not in ALLOWED_ARCHIVE_HOSTS:
        raise NativeBuildError("source archive URL is outside official GitHub endpoints")
    opener = urllib.request.build_opener(RestrictedRedirectHandler())
    request = urllib.request.Request(
        source["archive_url"],
        headers={"User-Agent": "telegram-cli-native-pin/1", "Accept-Encoding": "identity"},
    )
    temporary_path: Path | None = None
    try:
        descriptor, name = tempfile.mkstemp(prefix=".source-", dir=archive_directory)
        temporary_path = Path(name)
        digest = hashlib.sha256()
        received = 0
        started = time.monotonic()
        with os.fdopen(descriptor, "wb") as output:
            with opener.open(request, timeout=15) as response:
                final = urlparse(response.geturl())
                if final.scheme != "https" or final.hostname not in ALLOWED_ARCHIVE_HOSTS:
                    raise NativeBuildError("archive response escaped allowed GitHub hosts")
                content_length = response.headers.get("Content-Length")
                if (
                    content_length is not None
                    and content_length != str(source["archive_bytes"])
                ):
                    raise NativeBuildError(
                        "archive Content-Length differs from exact source contract"
                    )
                while True:
                    if time.monotonic() - started > limits["download_seconds"]:
                        raise NativeBuildError("source archive download deadline exceeded")
                    chunk = response.read(1024 * 1024)
                    if not chunk:
                        break
                    received += len(chunk)
                    if received > limits["source_archive_bytes"]:
                        raise NativeBuildError("source archive download exceeded hard cap")
                    digest.update(chunk)
                    output.write(chunk)
            output.flush()
            os.fsync(output.fileno())
        if received != source["archive_bytes"]:
            raise NativeBuildError(
                f"source archive bytes differ: {received} != {source['archive_bytes']}"
            )
        if digest.hexdigest() != source["archive_sha256"]:
            raise NativeBuildError("source archive SHA-256 differs from exact pin")
        temporary_path.chmod(0o444)
        os.replace(temporary_path, archive_path)
        directory_descriptor = os.open(archive_directory, os.O_RDONLY)
        try:
            os.fsync(directory_descriptor)
        finally:
            os.close(directory_descriptor)
    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)
    return archive_path, False


def _first_line(
    command: list[str],
    *,
    scratch_root: Path,
    work_id: str,
    build_lease: BuildLease,
) -> str:
    output = capture_command(
        command,
        timeout_seconds=20,
        maximum_rss_bytes=512 * 1024 * 1024,
        scratch_root=scratch_root,
        work_id=work_id,
        keepalive_fds=(build_lease.descriptor,),
    )
    first = output.splitlines()[0] if output else ""
    if not first:
        raise NativeBuildError(f"tool returned empty version output: {command[0]}")
    return first


def preflight(
    policy: dict[str, Any],
    *,
    scratch_root: Path,
    work_id: str,
    build_lease: BuildLease,
) -> dict[str, Any]:
    if platform.system() != "Darwin" or platform.machine() != "arm64":
        raise NativeBuildError("native target requires a macOS arm64 host")
    tools = {
        name: _tool_path(name)
        for name in ("cmake", "make", "clang", "gperf", "xcrun")
    }
    sdk_text = capture_command(
        [tools["xcrun"], "--show-sdk-path"],
        timeout_seconds=20,
        maximum_rss_bytes=512 * 1024 * 1024,
        scratch_root=scratch_root,
        work_id=work_id,
        keepalive_fds=(build_lease.descriptor,),
    )
    sdk_path = Path(sdk_text)
    if not sdk_path.is_dir() or str(sdk_path).startswith(("/private/tmp/", "/tmp/")):
        raise NativeBuildError("xcrun returned an invalid macOS SDK path")
    for required in (sdk_path / "usr/lib/libz.tbd", sdk_path / "usr/include/zlib.h"):
        if not required.is_file():
            raise NativeBuildError(f"macOS SDK zlib input is missing: {required.name}")

    openssl_prefix = Path("/opt/homebrew/opt/openssl@3")
    if not openssl_prefix.is_symlink():
        raise NativeBuildError("Homebrew openssl@3 opt link is absent")
    resolved_openssl = openssl_prefix.resolve(strict=True)
    if Path("/opt/homebrew/Cellar/openssl@3") not in resolved_openssl.parents:
        raise NativeBuildError("Homebrew openssl@3 opt link resolves outside its Cellar")
    if resolved_openssl.is_symlink() or not resolved_openssl.is_dir():
        raise NativeBuildError("resolved Homebrew openssl@3 Cellar path is invalid")
    libssl = resolved_openssl / "lib/libssl.a"
    libcrypto = resolved_openssl / "lib/libcrypto.a"
    include = resolved_openssl / "include"
    if not include.is_dir():
        raise NativeBuildError("Homebrew openssl@3 include directory is absent")
    libssl_sha, libssl_bytes = sha256_file(libssl, 16 * 1024 * 1024, "libssl.a")
    libcrypto_sha, libcrypto_bytes = sha256_file(
        libcrypto, 16 * 1024 * 1024, "libcrypto.a"
    )
    first_line_arguments = {
        "scratch_root": scratch_root,
        "work_id": work_id,
        "build_lease": build_lease,
    }
    openssl_version = _first_line(
        [str(resolved_openssl / "bin/openssl"), "version"],
        **first_line_arguments,
    )

    return {
        "paths": {
            **tools,
            "sdk": sdk_path,
            "openssl": resolved_openssl,
            "libssl": libssl,
            "libcrypto": libcrypto,
            "openssl_include": include,
            "zlib_library": sdk_path / "usr/lib/libz.tbd",
            "zlib_include": sdk_path / "usr/include",
        },
        "host": {
            "system": platform.system(),
            "release": platform.release(),
            "machine": platform.machine(),
        },
        "toolchain": {
            "cmake": _first_line([tools["cmake"], "--version"], **first_line_arguments),
            "make": _first_line([tools["make"], "--version"], **first_line_arguments),
            "clang": _first_line([tools["clang"], "--version"], **first_line_arguments),
            "gperf": _first_line([tools["gperf"], "--version"], **first_line_arguments),
            "python": f"Python {platform.python_version()}",
            "xcode_sdk": str(sdk_path),
        },
        "dependencies": {
            "openssl": {
                "formula": "openssl@3",
                "version": openssl_version,
                "prefix": str(openssl_prefix),
                "cellar_path": str(resolved_openssl),
                "linkage": "static",
                "libssl_sha256": libssl_sha,
                "libssl_bytes": libssl_bytes,
                "libcrypto_sha256": libcrypto_sha,
                "libcrypto_bytes": libcrypto_bytes,
            },
            "zlib": {"provider": "macos-sdk", "linkage": "system"},
        },
    }


def verify_static_openssl_archives(
    paths: dict[str, Path], dependency: dict[str, Any]
) -> None:
    """Доказывает, что exact static archives не изменились после build."""
    for name in ("libssl", "libcrypto"):
        digest, size = sha256_file(
            paths[name], 16 * 1024 * 1024, f"post-build {name}.a"
        )
        if (digest, size) != (
            dependency[f"{name}_sha256"],
            dependency[f"{name}_bytes"],
        ):
            raise NativeBuildError(f"static OpenSSL input changed during build: {name}.a")


def verify_extracted_sources(
    source_root: Path, schema_manifest: dict[str, Any]
) -> None:
    caps = {"cmake": 128 * 1024, "schema": 2 * 1024 * 1024, "license": 16 * 1024}
    for label in ("cmake", "schema", "license"):
        record = schema_manifest[label]
        digest, size = sha256_file(
            source_root / record["source_path"], caps[label], f"extracted {label}"
        )
        if (digest, size) != (record["sha256"], record["bytes"]):
            raise NativeBuildError(f"extracted {label} differs from vendored exact pin")


def inject_exact_git_head(source_root: Path, commit: str) -> dict[str, str]:
    """Даёт upstream CMake exact detached HEAD без поддельного Git object DB."""
    if (
        len(commit) != 40
        or any(character not in "0123456789abcdef" for character in commit)
    ):
        raise NativeBuildError("exact TDLib commit is not a lowercase SHA-1")
    metadata = source_root.lstat()
    if not stat.S_ISDIR(metadata.st_mode) or source_root.is_symlink():
        raise NativeBuildError("source root must be a regular directory")
    git_directory = source_root / ".git"
    if git_directory.exists() or git_directory.is_symlink():
        raise NativeBuildError("source archive unexpectedly contains .git metadata")
    git_directory.mkdir(mode=0o700)
    try:
        head = git_directory / "HEAD"
        atomic_write(head, f"{commit}\n".encode("ascii"), 0o444)
        head_sha, head_bytes = sha256_file(head, 1024, "synthetic detached HEAD")
        if head_bytes != 41:
            raise NativeBuildError("synthetic detached HEAD has unexpected size")
    except BaseException:
        shutil.rmtree(git_directory)
        raise
    return {
        "strategy": "synthetic-detached-head",
        "commit": commit,
        "head_sha256": head_sha,
    }


def verify_generated_commit_hash(
    template_path: Path, generated_path: Path, commit: str
) -> dict[str, str]:
    template = read_bounded(template_path, 64 * 1024, "GitCommitHash.cpp.in")
    token = b"@TD_GIT_COMMIT_HASH@"
    if template.count(token) != 1:
        raise NativeBuildError("GitCommitHash template token count differs")
    expected = template.replace(token, commit.encode("ascii"))
    generated = read_bounded(generated_path, 64 * 1024, "GitCommitHash.cpp")
    if generated != expected:
        raise NativeBuildError("generated GitCommitHash.cpp does not contain exact commit")
    return {
        "template_sha256": hashlib.sha256(template).hexdigest(),
        "generated_sha256": hashlib.sha256(generated).hexdigest(),
    }


def assert_build_inputs_unchanged(
    policy: dict[str, Any],
    schema_manifest: dict[str, Any],
    recipe_snapshot: dict[str, str],
) -> None:
    current_policy, current_manifest = load_exact_contracts()
    if current_policy != policy or current_manifest != schema_manifest:
        raise NativeBuildError("exact policy/schema changed during native build")
    if recipe_fingerprints() != recipe_snapshot:
        raise NativeBuildError("native build recipe changed during execution")


def _phase(
    name: str,
    command: list[str],
    *,
    work_root: Path,
    environment: dict[str, str],
    log_path: Path,
    timeout_seconds: int,
    policy: dict[str, Any],
    work_id: str,
    build_lease: BuildLease,
) -> dict[str, Any]:
    print(f"tdlib native build: {name} started", flush=True)
    try:
        metrics = run_guarded(
            command,
            cwd=work_root,
            environment=environment,
            log_path=log_path,
            timeout_seconds=timeout_seconds,
            maximum_tree_bytes=policy["limits"]["build_tree_bytes"],
            maximum_group_rss_bytes=policy["limits"]["process_group_rss_bytes"],
            maximum_group_processes=policy["limits"]["process_group_processes"],
            maximum_log_bytes=policy["limits"]["log_bytes"],
            termination_grace_seconds=policy["limits"]["termination_grace_seconds"],
            poll_seconds=1,
            work_id=work_id,
            keepalive_fds=(build_lease.descriptor,),
        )
    except BuildGuardError as error:
        try:
            tail = read_bounded(
                log_path, policy["limits"]["log_bytes"], f"{name} log"
            ).decode("utf-8", errors="replace").splitlines()[-20:]
        except (OSError, ValueError):
            tail = []
        detail = "\n".join(tail)
        suffix = f"; bounded log tail:\n{detail}" if detail else ""
        raise NativeBuildError(f"{name} failed: {error}{suffix}") from error
    log_sha, _ = sha256_file(log_path, policy["limits"]["log_bytes"], f"{name} log")
    print(
        f"tdlib native build: {name} completed "
        f"(seconds={metrics.duration_seconds:.1f}, "
        f"peak_rss_mib={metrics.peak_group_rss_bytes / 1024 / 1024:.1f}, "
        f"peak_tree_mib={metrics.peak_tree_bytes / 1024 / 1024:.1f})",
        flush=True,
    )
    return metrics.provenance_record(log_sha)


def find_exact_artifact(build_root: Path, basename: str) -> Path:
    candidates = [
        path
        for path in build_root.rglob(basename)
        if path.exists()
        and not path.is_symlink()
        and stat.S_ISREG(path.lstat().st_mode)
    ]
    if len(candidates) != 1:
        raise NativeBuildError(
            f"expected exactly one regular {basename}, found {len(candidates)}"
        )
    return candidates[0]


def build_provenance(
    *,
    policy: dict[str, Any],
    preflight_record: dict[str, Any],
    archive_reused: bool,
    extracted_bytes: int,
    extracted_members: int,
    commit_identity: dict[str, str],
    phases: dict[str, Any],
    inspection: dict[str, Any],
    recipe_snapshot: dict[str, str],
) -> dict[str, Any]:
    digest = inspection["sha256"]
    cache_path = artifact_cache_path(policy, digest).relative_to(ROOT)
    return {
        "format_version": 1,
        "source": policy["source"],
        "policy_sha256": canonical_sha256(policy),
        "recipe": {"files": recipe_snapshot},
        "target": expected_target_record(policy),
        "build": {
            "built_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
            "host": preflight_record["host"],
            "toolchain": preflight_record["toolchain"],
            "dependencies": preflight_record["dependencies"],
            "source_preparation": {
                "archive_sha256": policy["source"]["archive_sha256"],
                "archive_bytes": policy["source"]["archive_bytes"],
                "archive_cache_reused": archive_reused,
                "extracted_logical_bytes": extracted_bytes,
                "extracted_members": extracted_members,
                "commit_identity": commit_identity,
            },
            "phases": phases,
        },
        "artifact": {
            "cache_path": str(cache_path),
            "sha256": digest,
            "bytes": inspection["bytes"],
        },
        "verification": inspection["verification"],
        "reproducibility": {
            "status": "not_verified",
            "independent_builds": 1,
            "claim": (
                "exact source and bounded local recipe only; bit-for-bit "
                "reproducibility is not established"
            ),
        },
    }


def _prune_artifact_cache(policy: dict[str, Any], keep_digest: str) -> None:
    base = (ROOT / policy["target"]["artifact_cache_directory"]).resolve()
    native_root = NATIVE_ROOT.resolve()
    if base == native_root or native_root not in base.parents:
        raise NativeBuildError("refusing to prune artifact cache outside native root")
    if not base.exists():
        return
    for entry in base.iterdir():
        if not entry.is_dir() or entry.is_symlink():
            continue
        name = entry.name
        if (
            name != keep_digest
            and len(name) == 64
            and all(character in "0123456789abcdef" for character in name)
        ):
            shutil.rmtree(entry)


def try_reuse_local_artifact(
    policy: dict[str, Any], build_lease: BuildLease
) -> bool:
    if not PROVENANCE_PATH.exists() or PROVENANCE_PATH.is_symlink():
        return False
    with owned_work_directory(
        NATIVE_ROOT,
        cleanup_seconds=policy["limits"]["stale_cleanup_seconds"],
        maximum_entries=policy["limits"]["stale_cleanup_entries"],
        maximum_guard_states=policy["limits"]["stale_guard_states"],
    ) as work:
        try:
            provenance = read_json_bounded(
                PROVENANCE_PATH, 128 * 1024, "native provenance"
            )
            errors = provenance_errors(provenance, policy)
            if errors:
                return False
            digest = provenance["artifact"]["sha256"]
            artifact = artifact_cache_path(policy, digest)
            inspection = inspect_artifact(
                artifact,
                policy,
                scratch_root=work.path,
                work_id=work.work_id,
                keepalive_fds=(build_lease.descriptor,),
            )
            if local_artifact_errors(provenance, inspection):
                return False
        except (OSError, ValueError, NativeBuildError):
            return False
    _prune_artifact_cache(policy, digest)
    print(
        "tdlib native build: reused verified artifact "
        f"(sha256={digest}, bytes={inspection['bytes']})",
        flush=True,
    )
    return True


def perform_build(
    policy: dict[str, Any],
    schema_manifest: dict[str, Any],
    recipe_snapshot: dict[str, str],
    *,
    offline: bool,
    build_lease: BuildLease,
) -> None:
    with owned_work_directory(
        NATIVE_ROOT,
        cleanup_seconds=policy["limits"]["stale_cleanup_seconds"],
        maximum_entries=policy["limits"]["stale_cleanup_entries"],
        maximum_guard_states=policy["limits"]["stale_guard_states"],
    ) as work:
        work_directory = work.path
        preflight_record = preflight(
            policy,
            scratch_root=work_directory,
            work_id=work.work_id,
            build_lease=build_lease,
        )
        archive, archive_reused = download_source_archive(policy, offline=offline)
        print(
            "tdlib native build: exact source ready "
            f"(cache_reused={str(archive_reused).lower()}, "
            f"bytes={policy['source']['archive_bytes']})",
            flush=True,
        )
        staged_archive = work_directory / "inputs/source.tar.gz"
        stage_verified_file(
            archive,
            staged_archive,
            expected_sha256=policy["source"]["archive_sha256"],
            expected_bytes=policy["source"]["archive_bytes"],
            maximum_bytes=policy["limits"]["source_archive_bytes"],
        )
        source_root, extracted_bytes, extracted_members = safe_extract(
            staged_archive,
            work_directory / "source",
            maximum_bytes=policy["limits"]["extracted_source_bytes"],
            expected_root=policy["source"]["archive_root"],
            maximum_members=policy["limits"]["source_archive_members"],
            maximum_member_bytes=policy["limits"]["source_archive_member_bytes"],
            maximum_path_bytes=policy["limits"]["source_archive_path_bytes"],
            maximum_path_depth=policy["limits"]["source_archive_path_depth"],
        )
        verify_extracted_sources(source_root, schema_manifest)
        commit_identity = inject_exact_git_head(
            source_root, policy["source"]["commit"]
        )
        if (
            commit_identity["strategy"]
            != policy["source"]["commit_identity_strategy"]
            or commit_identity["head_sha256"]
            != policy["source"]["git_head_sha256"]
        ):
            raise NativeBuildError("synthetic detached HEAD differs from exact policy")
        build_root = work_directory / "build"
        build_root.mkdir(mode=0o700)
        paths = preflight_record["paths"]
        verify_static_openssl_archives(
            paths, preflight_record["dependencies"]["openssl"]
        )
        environment = minimal_environment(work_directory, paths["sdk"])
        resolved_defines = [f"-D{value}" for value in policy["target"]["cmake_defines"]]
        resolved_defines.extend(
            [
                f"-DCMAKE_OSX_SYSROOT={paths['sdk']}",
                f"-DCMAKE_MAKE_PROGRAM={paths['make']}",
                f"-DOPENSSL_ROOT_DIR={paths['openssl']}",
                f"-DOPENSSL_SSL_LIBRARY={paths['libssl']}",
                f"-DOPENSSL_CRYPTO_LIBRARY={paths['libcrypto']}",
                f"-DOPENSSL_INCLUDE_DIR={paths['openssl_include']}",
                f"-DZLIB_LIBRARY={paths['zlib_library']}",
                f"-DZLIB_INCLUDE_DIR={paths['zlib_include']}",
            ]
        )
        phases = {
            "configure": _phase(
                "configure",
                [
                    paths["cmake"],
                    "-S",
                    str(source_root),
                    "-B",
                    str(build_root),
                    "-G",
                    policy["target"]["cmake_generator"],
                    *resolved_defines,
                ],
                work_root=work_directory,
                environment=environment,
                log_path=work_directory / "configure.log",
                timeout_seconds=policy["limits"]["configure_seconds"],
                policy=policy,
                work_id=work.work_id,
                build_lease=build_lease,
            )
        }
        commit_hashes = verify_generated_commit_hash(
            source_root / "td/telegram/GitCommitHash.cpp.in",
            build_root / "td/telegram/GitCommitHash.cpp",
            policy["source"]["commit"],
        )
        if (
            commit_hashes["template_sha256"]
            != policy["source"]["git_commit_template_sha256"]
            or commit_hashes["generated_sha256"]
            != policy["source"]["git_commit_generated_sha256"]
        ):
            raise NativeBuildError("generated commit source differs from exact policy")
        configure_log = read_bounded(
            work_directory / "configure.log",
            policy["limits"]["log_bytes"],
            "configure log",
        ).decode("utf-8", errors="strict")
        if f"-- Git state: {policy['source']['commit']}" not in configure_log.splitlines():
            raise NativeBuildError("configure log did not confirm exact Git state")
        commit_identity.update(commit_hashes)
        assert_build_inputs_unchanged(policy, schema_manifest, recipe_snapshot)
        phases["build"] = _phase(
            "build",
            [
                paths["cmake"],
                "--build",
                str(build_root),
                "--target",
                policy["target"]["cmake_target"],
                "--parallel",
                str(policy["limits"]["parallel_jobs"]),
            ],
            work_root=work_directory,
            environment=environment,
            log_path=work_directory / "build.log",
            timeout_seconds=policy["limits"]["build_seconds"],
            policy=policy,
            work_id=work.work_id,
            build_lease=build_lease,
        )
        verify_static_openssl_archives(
            paths, preflight_record["dependencies"]["openssl"]
        )
        built_artifact = find_exact_artifact(
            build_root, policy["target"]["artifact_basename"]
        )
        assert_build_inputs_unchanged(policy, schema_manifest, recipe_snapshot)
        inspection_arguments = {
            "scratch_root": work_directory,
            "work_id": work.work_id,
            "keepalive_fds": (build_lease.descriptor,),
        }
        inspection = inspect_artifact(
            built_artifact, policy, **inspection_arguments
        )
        assert_build_inputs_unchanged(policy, schema_manifest, recipe_snapshot)
        provenance = build_provenance(
            policy=policy,
            preflight_record=preflight_record,
            archive_reused=archive_reused,
            extracted_bytes=extracted_bytes,
            extracted_members=extracted_members,
            commit_identity=commit_identity,
            phases=phases,
            inspection=inspection,
            recipe_snapshot=recipe_snapshot,
        )
        errors = provenance_errors(provenance, policy)
        if errors:
            raise NativeBuildError("generated provenance rejected: " + "; ".join(errors))

        digest = inspection["sha256"]
        cache_path = artifact_cache_path(policy, digest)
        copied_digest, copied_bytes = atomic_copy(
            cache_path, built_artifact, policy["limits"]["artifact_bytes"]
        )
        if (copied_digest, copied_bytes) != (digest, inspection["bytes"]):
            raise NativeBuildError("published artifact differs from inspected staging")
        cached_inspection = inspect_artifact(
            cache_path, policy, **inspection_arguments
        )
        if cached_inspection != inspection:
            raise NativeBuildError("cached artifact verification differs from staging")
        assert_build_inputs_unchanged(policy, schema_manifest, recipe_snapshot)
        payload = (
            json.dumps(provenance, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
        ).encode("utf-8")
        atomic_write(PROVENANCE_PATH, payload, 0o644)
        _prune_artifact_cache(policy, digest)
        print(
            "tdlib native build: published exact artifact provenance "
            f"(sha256={digest}, bytes={copied_bytes}, jobs=2, "
            "reproducibility=not_verified)",
            flush=True,
        )
def main() -> int:
    arguments = parse_arguments()
    policy, schema_manifest = load_exact_contracts()
    NATIVE_ROOT.mkdir(parents=True, exist_ok=True, mode=0o700)
    native_metadata = NATIVE_ROOT.lstat()
    if not stat.S_ISDIR(native_metadata.st_mode) or native_metadata.st_uid != os.getuid():
        raise NativeBuildError("native root must be an owned directory without symlink")
    NATIVE_ROOT.chmod(0o700)
    with termination_signals_as_errors(), exclusive_build_lock(
        wait_seconds=policy["limits"]["build_lock_wait_seconds"]
    ) as build_lease:
        cleanup_stale_work_directories(
            build_lease=build_lease,
            native_root=NATIVE_ROOT,
            cleanup_seconds=policy["limits"]["stale_cleanup_seconds"],
            maximum_directories=policy["limits"]["stale_work_directories"],
            maximum_guard_states=policy["limits"]["stale_guard_states"],
            maximum_entries=policy["limits"]["stale_cleanup_entries"],
        )
        if not arguments.force and try_reuse_local_artifact(policy, build_lease):
            return 0
        recipe_snapshot = recipe_fingerprints()
        perform_build(
            policy,
            schema_manifest,
            recipe_snapshot,
            offline=arguments.offline,
            build_lease=build_lease,
        )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, NativeBuildError) as error:
        print(f"tdlib native build: {error}", file=sys.stderr)
        raise SystemExit(1) from error
