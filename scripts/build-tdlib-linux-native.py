#!/usr/bin/env python3
"""Собирает pinned TDJSON для Linux x86_64 в exact Docker builder."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import time
from typing import Any
from urllib.parse import urlparse
import urllib.request

from tdlib_linux_native import (
    LINUX_DOCKERFILE_PATH,
    LINUX_PROVENANCE_PATH,
    inspect_linux_artifact,
    linux_artifact_cache_path,
    linux_executor_record,
    linux_local_artifact_errors,
    linux_provenance_errors,
    linux_recipe_fingerprints,
    load_linux_contracts,
    verify_builder_image,
    expected_linux_target_record,
)
from tdlib_native import (
    MAX_PROVENANCE_BYTES,
    MAX_SCHEMA_MANIFEST_BYTES,
    NATIVE_ROOT,
    ROOT,
    SCHEMA_MANIFEST_PATH,
    NativeBuildError,
    atomic_copy,
    atomic_write,
    canonical_sha256,
    cleanup_stale_work_directories,
    exclusive_build_lock,
    owned_work_directory,
    read_bounded,
    read_json_bounded,
    safe_extract,
    sha256_file,
)


ALLOWED_ARCHIVE_HOSTS = {"github.com", "codeload.github.com"}
CONTAINER_LABEL = "com.lonmstalker.telegram-cli.tdlib-linux-build"


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--offline", action="store_true")
    return parser.parse_args()


def run_command(
    command: list[str], *, timeout: int, output: int | Any = subprocess.PIPE
) -> subprocess.CompletedProcess[Any]:
    try:
        result = subprocess.run(
            command,
            check=False,
            stdout=output,
            stderr=subprocess.STDOUT if output != subprocess.PIPE else subprocess.PIPE,
            text=output == subprocess.PIPE,
            timeout=timeout,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise NativeBuildError(f"command failed to run: {command[0]} {command[1]}") from error
    if result.returncode != 0:
        detail = ""
        if isinstance(result.stderr, str):
            detail = result.stderr.strip().splitlines()[-1] if result.stderr.strip() else ""
        raise NativeBuildError(
            f"command failed ({result.returncode}): {command[0]} {command[1]}"
            + (f": {detail}" if detail else "")
        )
    return result


def command_output(command: list[str], *, timeout: int = 30) -> str:
    result = run_command(command, timeout=timeout)
    output = str(result.stdout).strip()
    if len(output.encode("utf-8")) > 2 * 1024 * 1024:
        raise NativeBuildError("command output exceeds cap")
    return output


def ensure_builder_image(policy: dict[str, Any], *, offline: bool) -> None:
    try:
        verify_builder_image(policy)
        return
    except NativeBuildError:
        if offline:
            raise NativeBuildError("offline mode: exact Linux builder image is absent")
    run_command(
        [
            "docker", "build", "--quiet", "--platform", policy["builder"]["platform"],
            "--file", str(LINUX_DOCKERFILE_PATH), "--tag",
            policy["builder"]["image_tag"], str(ROOT),
        ],
        timeout=900,
    )
    verify_builder_image(policy)


def source_archive(
    source: dict[str, Any], policy: dict[str, Any], *, offline: bool
) -> tuple[Path, bool]:
    directory = NATIVE_ROOT / "downloads"
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / f"{source['archive_sha256']}.tar.gz"
    if path.exists() and not path.is_symlink():
        try:
            actual = sha256_file(
                path,
                policy["limits"]["source_archive_bytes"],
                "TDLib source archive",
            )
            if actual == (source["archive_sha256"], source["archive_bytes"]):
                return path, True
        except (OSError, ValueError):
            pass
    if offline:
        raise NativeBuildError("offline mode: exact TDLib source archive is absent")
    parsed = urlparse(source["archive_url"])
    if parsed.scheme != "https" or parsed.hostname not in ALLOWED_ARCHIVE_HOSTS:
        raise NativeBuildError("TDLib archive URL is outside official GitHub endpoints")
    temporary = directory / f".download-{os.getpid()}"
    digest = hashlib.sha256()
    received = 0
    started = time.monotonic()
    try:
        request = urllib.request.Request(
            source["archive_url"],
            headers={"User-Agent": "telegram-cli-linux-native-pin/1", "Accept-Encoding": "identity"},
        )
        with urllib.request.urlopen(request, timeout=15) as response, temporary.open("xb") as output:
            final = urlparse(response.geturl())
            if final.scheme != "https" or final.hostname not in ALLOWED_ARCHIVE_HOSTS:
                raise NativeBuildError("TDLib archive redirect escaped official GitHub endpoints")
            while True:
                if time.monotonic() - started > 120:
                    raise NativeBuildError("TDLib archive download deadline exceeded")
                chunk = response.read(1024 * 1024)
                if not chunk:
                    break
                received += len(chunk)
                if received > policy["limits"]["source_archive_bytes"]:
                    raise NativeBuildError("TDLib archive download exceeds cap")
                digest.update(chunk)
                output.write(chunk)
            output.flush()
            os.fsync(output.fileno())
        if (digest.hexdigest(), received) != (source["archive_sha256"], source["archive_bytes"]):
            raise NativeBuildError("downloaded TDLib archive differs from exact source")
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)
    return path, False


def verify_extracted_sources(source_root: Path, manifest: dict[str, Any]) -> None:
    caps = {"cmake": 128 * 1024, "schema": 2 * 1024 * 1024, "license": 16 * 1024}
    for label, maximum in caps.items():
        record = manifest[label]
        actual = sha256_file(source_root / record["source_path"], maximum, f"extracted {label}")
        if actual != (record["sha256"], record["bytes"]):
            raise NativeBuildError(f"extracted {label} differs from vendored exact pin")


def inject_exact_git_head(source_root: Path, source: dict[str, Any]) -> dict[str, str]:
    commit = source["commit"]
    if len(commit) != 40 or any(character not in "0123456789abcdef" for character in commit):
        raise NativeBuildError("exact TDLib commit is not a lowercase SHA-1")
    git_directory = source_root / ".git"
    if git_directory.exists() or git_directory.is_symlink():
        raise NativeBuildError("source archive unexpectedly contains .git metadata")
    git_directory.mkdir(mode=0o700)
    head = git_directory / "HEAD"
    atomic_write(head, f"{commit}\n".encode("ascii"), 0o444)
    digest, size = sha256_file(head, 1024, "synthetic detached HEAD")
    if size != 41 or digest != source["git_head_sha256"]:
        raise NativeBuildError("synthetic detached HEAD differs from exact source contract")
    return {"strategy": source["commit_identity_strategy"], "commit": commit, "head_sha256": digest}


def verify_generated_commit(template: Path, generated: Path, source: dict[str, Any]) -> dict[str, str]:
    template_bytes = read_bounded(template, 64 * 1024, "GitCommitHash.cpp.in")
    token = b"@TD_GIT_COMMIT_HASH@"
    if template_bytes.count(token) != 1:
        raise NativeBuildError("GitCommitHash template token count differs")
    generated_bytes = read_bounded(generated, 64 * 1024, "GitCommitHash.cpp")
    if generated_bytes != template_bytes.replace(token, source["commit"].encode("ascii")):
        raise NativeBuildError("generated GitCommitHash.cpp differs from exact commit")
    result = {
        "template_sha256": hashlib.sha256(template_bytes).hexdigest(),
        "generated_sha256": hashlib.sha256(generated_bytes).hexdigest(),
    }
    if result != {
        "template_sha256": source["git_commit_template_sha256"],
        "generated_sha256": source["git_commit_generated_sha256"],
    }:
        raise NativeBuildError("generated commit source hashes differ")
    return result


def cleanup_build_containers() -> None:
    ids = command_output(
        ["docker", "ps", "-aq", "--filter", f"label={CONTAINER_LABEL}=1"]
    ).splitlines()
    for container_id in ids:
        run_command(["docker", "rm", "-f", container_id], timeout=30)


def phase(
    name: str, command: list[str], *, log_path: Path, timeout: int, container: str
) -> dict[str, Any]:
    print(f"tdlib Linux native build: {name} started", flush=True)
    started = time.monotonic()
    try:
        with log_path.open("xb") as output:
            result = run_command(command, timeout=timeout, output=output)
    except BaseException:
        subprocess.run(
            ["docker", "rm", "-f", container], check=False,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=30,
        )
        try:
            tail = read_bounded(
                log_path, 16 * 1024**2, f"{name} log"
            ).decode("utf-8", errors="replace").splitlines()[-20:]
        except (OSError, ValueError):
            tail = []
        if tail:
            print("\n".join(tail), file=sys.stderr)
        raise
    duration = time.monotonic() - started
    digest, size = sha256_file(log_path, 16 * 1024**2, f"{name} log")
    print(f"tdlib Linux native build: {name} completed (seconds={duration:.1f})", flush=True)
    return {
        "duration_seconds": round(duration, 3),
        "log_retained": True,
        "log_bytes": size,
        "log_sha256": digest,
        "return_code": result.returncode,
    }


def container_packages(container: str, policy: dict[str, Any]) -> dict[str, str]:
    names = list(policy["builder"]["packages"])
    output = command_output(["docker", "exec", container, "dpkg-query", "-W", *names])
    packages: dict[str, str] = {}
    for line in output.splitlines():
        try:
            name, version = line.split("\t", 1)
        except ValueError as error:
            raise NativeBuildError("builder package inventory is malformed") from error
        packages[name] = version
    if packages != policy["builder"]["packages"]:
        raise NativeBuildError("builder package inventory differs from exact policy")
    return packages


def start_build_container(policy: dict[str, Any], work_id: str) -> str:
    name = f"telegram-cli-tdlib-linux-build-{work_id}"
    limits = policy["limits"]
    run_command(
        [
            "docker", "run", "-d", "--name", name,
            "--label", f"{CONTAINER_LABEL}=1",
            "--platform", policy["builder"]["platform"], "--network", "none",
            "--cpus", str(limits["cpus"]), "--memory", str(limits["memory_bytes"]),
            "--pids-limit", str(limits["pids"]), policy["builder"]["image_id"],
            "sleep", "infinity",
        ],
        timeout=60,
    )
    return name


def prune_cache(policy: dict[str, Any], keep_digest: str) -> None:
    base = (ROOT / policy["target"]["artifact_cache_directory"]).resolve()
    if not base.exists():
        return
    for entry in base.iterdir():
        if (
            entry.is_dir()
            and not entry.is_symlink()
            and entry.name != keep_digest
            and len(entry.name) == 64
            and all(c in "0123456789abcdef" for c in entry.name)
        ):
            shutil.rmtree(entry)


def try_reuse(policy: dict[str, Any], source: dict[str, Any]) -> bool:
    if not LINUX_PROVENANCE_PATH.exists() or LINUX_PROVENANCE_PATH.is_symlink():
        return False
    try:
        provenance = read_json_bounded(LINUX_PROVENANCE_PATH, MAX_PROVENANCE_BYTES, "Linux provenance")
        if linux_provenance_errors(provenance, policy, source):
            return False
        artifact = linux_artifact_cache_path(policy, provenance["artifact"]["sha256"])
        inspection = inspect_linux_artifact(artifact, policy, source)
        if linux_local_artifact_errors(provenance, inspection):
            return False
    except (OSError, ValueError, NativeBuildError):
        return False
    prune_cache(policy, inspection["sha256"])
    print(f"tdlib Linux native build: reused verified artifact (sha256={inspection['sha256']})")
    return True


def perform_build(policy: dict[str, Any], source: dict[str, Any], *, offline: bool) -> None:
    manifest = read_json_bounded(SCHEMA_MANIFEST_PATH, MAX_SCHEMA_MANIFEST_BYTES, "schema manifest")
    recipe = linux_recipe_fingerprints()
    archive, archive_reused = source_archive(source, policy, offline=offline)
    with owned_work_directory(NATIVE_ROOT) as work:
        source_root, extracted_bytes, extracted_members = safe_extract(
            archive,
            work.path / "source",
            maximum_bytes=policy["limits"]["extracted_source_bytes"],
            expected_root=source["archive_root"],
            maximum_members=policy["limits"]["source_archive_members"],
            maximum_member_bytes=policy["limits"]["source_archive_member_bytes"],
            maximum_path_bytes=policy["limits"]["source_archive_path_bytes"],
            maximum_path_depth=policy["limits"]["source_archive_path_depth"],
        )
        verify_extracted_sources(source_root, manifest)
        commit_identity = inject_exact_git_head(source_root, source)
        container = start_build_container(policy, work.work_id)
        try:
            run_command(["docker", "exec", container, "mkdir", "-p", "/work/source", "/work/build"], timeout=30)
            run_command(["docker", "cp", f"{source_root}/.", f"{container}:/work/source"], timeout=120)
            packages = container_packages(container, policy)
            defines = [f"-D{value}" for value in policy["target"]["cmake_defines"]]
            phases = {
                "configure": phase(
                    "configure",
                    [
                        "docker", "exec", container, "cmake", "-S", "/work/source",
                        "-B", "/work/build", "-G", policy["target"]["cmake_generator"],
                        *defines,
                    ],
                    log_path=work.path / "configure.log",
                    timeout=policy["limits"]["configure_seconds"],
                    container=container,
                )
            }
            configure_log = read_bounded(work.path / "configure.log", 16 * 1024**2, "configure log").decode("utf-8")
            if f"-- Git state: {source['commit']}" not in configure_log.splitlines():
                raise NativeBuildError("configure log did not confirm exact Git state")
            phases["build"] = phase(
                "build",
                [
                    "docker", "exec", container, "cmake", "--build", "/work/build",
                    "--target", policy["target"]["cmake_target"], "--parallel",
                    str(policy["limits"]["cpus"]),
                ],
                log_path=work.path / "build.log",
                timeout=policy["limits"]["build_seconds"],
                container=container,
            )
            tree_bytes = int(
                command_output(
                    ["docker", "exec", container, "du", "-sb", "/work/build"]
                ).split()[0]
            )
            if tree_bytes > policy["limits"]["build_tree_bytes"]:
                raise NativeBuildError("Linux build tree exceeds exact cap")
            artifact_paths = command_output(
                [
                    "docker", "exec", container, "find", "/work/build", "-type",
                    "f", "-name", policy["target"]["artifact_basename"], "-print",
                ]
            ).splitlines()
            if len(artifact_paths) != 1:
                raise NativeBuildError(
                    f"expected one Linux artifact, found {len(artifact_paths)}"
                )
            template = work.path / "GitCommitHash.cpp.in"
            generated = work.path / "GitCommitHash.cpp"
            run_command(
                [
                    "docker", "cp",
                    f"{container}:/work/source/td/telegram/GitCommitHash.cpp.in",
                    str(template),
                ],
                timeout=30,
            )
            run_command(
                [
                    "docker", "cp",
                    f"{container}:/work/build/td/telegram/GitCommitHash.cpp",
                    str(generated),
                ],
                timeout=30,
            )
            commit_identity.update(verify_generated_commit(template, generated, source))
            artifact = work.path / policy["target"]["artifact_basename"]
            run_command(
                ["docker", "cp", f"{container}:{artifact_paths[0]}", str(artifact)],
                timeout=60,
            )
        finally:
            subprocess.run(
                ["docker", "rm", "-f", container], check=False,
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=30,
            )
        if not stat.S_ISREG(artifact.lstat().st_mode) or artifact.is_symlink():
            raise NativeBuildError("copied Linux artifact is not a regular file")
        inspection = inspect_linux_artifact(artifact, policy, source)
        current_policy, current_source = load_linux_contracts()
        if current_policy != policy or current_source != source or linux_recipe_fingerprints() != recipe:
            raise NativeBuildError("Linux build inputs changed during execution")
        digest = inspection["sha256"]
        cache_path = linux_artifact_cache_path(policy, digest)
        copied = atomic_copy(cache_path, artifact, policy["limits"]["artifact_bytes"])
        if copied != (digest, inspection["bytes"]):
            raise NativeBuildError("published Linux artifact differs from inspected artifact")
        cached_inspection = inspect_linux_artifact(cache_path, policy, source)
        if cached_inspection != inspection:
            raise NativeBuildError("cached Linux artifact verification differs")
        provenance = {
            "format_version": 1,
            "source": source,
            "policy_sha256": canonical_sha256(policy),
            "reviewed_recipe": {"files": recipe},
            "target": expected_linux_target_record(policy),
            "build": {
                "built_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
                "executor": linux_executor_record(),
                "builder": {
                    "image_id": policy["builder"]["image_id"],
                    "platform": policy["builder"]["platform"],
                    "packages": packages,
                },
                "source_preparation": {
                    "archive_sha256": source["archive_sha256"],
                    "archive_bytes": source["archive_bytes"],
                    "archive_cache_reused": archive_reused,
                    "extracted_logical_bytes": extracted_bytes,
                    "extracted_members": extracted_members,
                    "commit_identity": commit_identity,
                },
                "phases": phases,
            },
            "artifact": {
                "cache_path": str(cache_path.relative_to(ROOT)),
                "sha256": digest,
                "bytes": inspection["bytes"],
            },
            "verification": inspection["verification"],
            "reproducibility": {
                "status": "not_verified",
                "independent_builds": 1,
                "claim": (
                    "exact source and pinned builder observation only; bit-for-bit "
                    "reproducibility is not established"
                ),
            },
        }
        errors = linux_provenance_errors(provenance, policy, source)
        if errors:
            raise NativeBuildError(
                "generated Linux provenance rejected: " + "; ".join(errors)
            )
        atomic_write(
            LINUX_PROVENANCE_PATH,
            (json.dumps(provenance, indent=2, sort_keys=True) + "\n").encode("utf-8"),
            0o644,
        )
        prune_cache(policy, digest)
        print(
            "tdlib Linux native build: published exact artifact provenance "
            f"(sha256={digest}, bytes={inspection['bytes']}, reproducibility=not_verified)"
        )


def main() -> int:
    arguments = parse_arguments()
    policy, source = load_linux_contracts()
    NATIVE_ROOT.mkdir(parents=True, exist_ok=True, mode=0o700)
    with exclusive_build_lock(wait_seconds=30) as lease:
        cleanup_stale_work_directories(
            build_lease=lease,
            native_root=NATIVE_ROOT,
            cleanup_seconds=60,
            maximum_directories=2,
            maximum_guard_states=2,
            maximum_entries=500_000,
        )
        cleanup_build_containers()
        ensure_builder_image(policy, offline=arguments.offline)
        if not arguments.force and try_reuse(policy, source):
            return 0
        perform_build(policy, source, offline=arguments.offline)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, NativeBuildError) as error:
        print(f"tdlib Linux native build: {error}", file=sys.stderr)
        raise SystemExit(1) from error
