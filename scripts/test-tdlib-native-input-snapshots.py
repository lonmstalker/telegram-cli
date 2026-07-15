#!/usr/bin/env python3
"""Negative controls для immutable archive/OpenSSL inputs native build."""

from __future__ import annotations

import hashlib
import importlib.util
import os
from pathlib import Path
import stat
import sys
import tempfile


ROOT = Path(__file__).resolve().parent.parent
BUILDER_PATH = ROOT / "scripts/build-tdlib-native.py"


def load_builder():
    spec = importlib.util.spec_from_file_location("tdlib_native_builder", BUILDER_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("native builder module не загружен")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main() -> int:
    builder = load_builder()
    required = ("stage_verified_file", "verify_static_openssl_archives")
    missing = [name for name in required if not hasattr(builder, name)]
    if missing:
        print(
            "tdlib native input test: отсутствуют " + ", ".join(missing),
            file=sys.stderr,
        )
        return 1

    with tempfile.TemporaryDirectory(prefix="tdlib-native-input-") as directory:
        root = Path(directory)
        payload = b"exact source archive\n"
        source = root / "source.tar.gz"
        source.write_bytes(payload)
        digest = hashlib.sha256(payload).hexdigest()
        staged = root / "private/staged.tar.gz"
        builder.stage_verified_file(
            source,
            staged,
            expected_sha256=digest,
            expected_bytes=len(payload),
            maximum_bytes=1024,
        )
        if staged.read_bytes() != payload:
            print("tdlib native input test: staged snapshot differs", file=sys.stderr)
            return 1
        if stat.S_IMODE(staged.lstat().st_mode) != 0o400:
            print("tdlib native input test: staged mode не 0400", file=sys.stderr)
            return 1

        wrong_destination = root / "private/wrong.tar.gz"
        try:
            builder.stage_verified_file(
                source,
                wrong_destination,
                expected_sha256="0" * 64,
                expected_bytes=len(payload),
                maximum_bytes=1024,
            )
        except builder.NativeBuildError:
            pass
        else:
            print("tdlib native input test: wrong archive hash принят", file=sys.stderr)
            return 1
        if wrong_destination.exists():
            print("tdlib native input test: wrong snapshot опубликован", file=sys.stderr)
            return 1

        symlink = root / "source-link"
        symlink.symlink_to(source)
        try:
            builder.stage_verified_file(
                symlink,
                root / "private/link.tar.gz",
                expected_sha256=digest,
                expected_bytes=len(payload),
                maximum_bytes=1024,
            )
        except (OSError, builder.NativeBuildError):
            pass
        else:
            print("tdlib native input test: symlink archive принят", file=sys.stderr)
            return 1

        cellar = root / "Cellar/openssl@3/3.6.2"
        library_directory = cellar / "lib"
        library_directory.mkdir(parents=True)
        libssl = library_directory / "libssl.a"
        libcrypto = library_directory / "libcrypto.a"
        libssl.write_bytes(b"ssl")
        libcrypto.write_bytes(b"crypto")
        paths = {"libssl": libssl, "libcrypto": libcrypto}
        dependency = {
            "libssl_sha256": hashlib.sha256(b"ssl").hexdigest(),
            "libssl_bytes": 3,
            "libcrypto_sha256": hashlib.sha256(b"crypto").hexdigest(),
            "libcrypto_bytes": 6,
        }
        builder.verify_static_openssl_archives(paths, dependency)
        libssl.write_bytes(b"changed")
        try:
            builder.verify_static_openssl_archives(paths, dependency)
        except builder.NativeBuildError:
            pass
        else:
            print("tdlib native input test: changed libssl.a принят", file=sys.stderr)
            return 1

    print("tdlib native input test: ok (snapshot=1, negative_controls=3)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
