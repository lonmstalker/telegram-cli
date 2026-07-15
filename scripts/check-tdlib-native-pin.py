#!/usr/bin/env python3
"""Проверяет exact native TDLib policy/provenance и опционально local dylib."""

from __future__ import annotations

import argparse
import copy
from pathlib import Path
import sys
import tempfile

from tdlib_native import (
    EXPECTED_POLICY,
    MAX_PROVENANCE_BYTES,
    NATIVE_ROOT,
    NativeBuildError,
    PROVENANCE_PATH,
    artifact_cache_path,
    clone_policy_with,
    inspect_artifact,
    load_exact_contracts,
    local_artifact_errors,
    provenance_errors,
    read_json_bounded,
    shared_artifact_lock,
    sha256_file,
    validate_policy_contract,
)


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--require-local-artifact",
        action="store_true",
        help="дополнительно проверить ignored local dylib и isolated smoke",
    )
    return parser.parse_args()


def negative_control_errors(
    policy: dict[str, object],
    schema_manifest: dict[str, object],
    provenance: dict[str, object],
) -> list[str]:
    errors: list[str] = []
    mutations = (
        ("commit", ("source", "commit"), "0" * 40),
        ("archive hash", ("source", "archive_sha256"), "0" * 64),
        ("target", ("target", "triple"), "x86_64-apple-darwin"),
        ("jobs", ("limits", "parallel_jobs"), 3),
        ("unsafe cache", ("target", "artifact_cache_directory"), "../escape"),
        ("inflated tree cap", ("limits", "build_tree_bytes"), 64 * 1024**3),
        ("bool limit", ("limits", "parallel_jobs"), True),
    )
    for label, path, value in mutations:
        candidate = clone_policy_with(policy, path, value)
        if not validate_policy_contract(candidate, schema_manifest):
            errors.append(f"negative control: policy {label} mutation not detected")

    provenance_mutations = (
        ("artifact hash", ("artifact", "sha256"), "0" * 64),
        ("artifact target", ("target", "triple"), "x86_64-apple-darwin"),
        ("runtime version", ("verification", "options", "version"), "9.9.9"),
        ("dynamic dependency", ("verification", "dynamic_dependencies", 0), "/tmp/libssl.dylib"),
        ("reproducibility", ("reproducibility", "status"), "verified"),
        (
            "commit injection",
            ("build", "source_preparation", "commit_identity", "commit"),
            "0" * 40,
        ),
        (
            "OpenSSL hash",
            ("build", "dependencies", "openssl", "libssl_sha256"),
            "z" * 64,
        ),
        (
            "OpenSSL Cellar path",
            ("build", "dependencies", "openssl", "cellar_path"),
            "/opt/homebrew/opt/openssl@3",
        ),
        ("phase log hash", ("build", "phases", "build", "log_sha256"), "z" * 64),
    )
    for label, path, value in provenance_mutations:
        candidate = copy.deepcopy(provenance)
        cursor: object = candidate
        for field in path[:-1]:
            cursor = cursor[field]  # type: ignore[index]
        cursor[path[-1]] = value  # type: ignore[index]
        if not provenance_errors(candidate, policy):
            errors.append(f"negative control: provenance {label} mutation not detected")

    NATIVE_ROOT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".pin-guard-", dir=NATIVE_ROOT) as directory:
        oversized = Path(directory) / "oversized.dylib"
        with oversized.open("wb") as output:
            output.truncate(policy["limits"]["artifact_bytes"] + 1)  # type: ignore[index]
        try:
            sha256_file(
                oversized,
                policy["limits"]["artifact_bytes"],  # type: ignore[index]
                "oversized artifact",
            )
        except ValueError:
            pass
        else:
            errors.append("negative control: oversized sparse artifact was hashed")
    return errors


def main() -> int:
    arguments = parse_arguments()
    policy, schema_manifest = load_exact_contracts()
    if policy != EXPECTED_POLICY:
        raise NativeBuildError("native policy differs from exact in-code contract")
    provenance = read_json_bounded(
        PROVENANCE_PATH, MAX_PROVENANCE_BYTES, "native provenance"
    )
    errors = provenance_errors(provenance, policy)
    errors.extend(negative_control_errors(policy, schema_manifest, provenance))
    if errors:
        raise NativeBuildError("; ".join(errors))

    digest = provenance["artifact"]["sha256"]
    if arguments.require_local_artifact:
        with shared_artifact_lock():
            artifact = artifact_cache_path(policy, digest)
            inspection = inspect_artifact(artifact, policy)
            errors = local_artifact_errors(provenance, inspection)
        if errors:
            raise NativeBuildError("; ".join(errors))
        print(
            "tdlib native pin: ok "
            f"(mode=artifact-verified, version={policy['source']['version']}, "
            f"target={policy['target']['triple']}, sha256={digest}, "
            "negative_controls=17)"
        )
    else:
        print(
            "tdlib native pin: ok "
            f"(mode=provenance-only, version={policy['source']['version']}, "
            f"target={policy['target']['triple']}, sha256={digest}, "
            "negative_controls=17)"
        )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (KeyError, OSError, TypeError, ValueError, NativeBuildError) as error:
        print(f"tdlib native pin: {error}", file=sys.stderr)
        raise SystemExit(1) from error
