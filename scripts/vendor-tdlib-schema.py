#!/usr/bin/env python3
"""Получает и публикует exact TDLib provenance/schema bundle с hard caps."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import stat
import tempfile
from typing import Any, Callable
import urllib.request


ROOT = Path(__file__).resolve().parent.parent
MANIFEST_PATH = ROOT / "vendor/tdlib/manifest.json"
RAW_BASE = "https://raw.githubusercontent.com/tdlib/td"
TIMEOUT_SECONDS = 30
MAX_MANIFEST_BYTES = 16 * 1024
PAYLOAD_CAPS = {
    "cmake": 128 * 1024,
    "schema": 2 * 1024 * 1024,
    "license": 16 * 1024,
}
EXPECTED_UPSTREAM = {
    "repository": "https://github.com/tdlib/td",
    "commit": "07d3a0973f5113b0827a04d54a93aaaa9e288348",
    "version": "1.8.66",
}
EXPECTED_MANIFEST_SHA256 = "9e56f4038adc11a744ecabe9ea45a78a0917c3bf3d6a2c5ff67f26824b5e9be4"
Downloader = Callable[[str, int, int], bytes]


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


def validate_vendor_manifest(manifest: dict[str, Any]) -> None:
    canonical = json.dumps(
        manifest, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")
    if hashlib.sha256(canonical).hexdigest() != EXPECTED_MANIFEST_SHA256:
        raise ValueError("TDLib manifest differs from exact vendor contract")
    if manifest.get("format_version") != 1:
        raise ValueError("unsupported TDLib pin manifest format")
    if manifest.get("upstream") != EXPECTED_UPSTREAM:
        raise ValueError("TDLib upstream identity differs from exact vendor policy")
    if set(manifest) != {"format_version", "upstream", "cmake", "schema", "license"}:
        raise ValueError("TDLib pin manifest has unexpected top-level fields")

    for label, cap in PAYLOAD_CAPS.items():
        record = manifest.get(label)
        if not isinstance(record, dict):
            raise ValueError(f"missing manifest record: {label}")
        expected_bytes = record.get("bytes")
        if not isinstance(expected_bytes, int) or not 0 < expected_bytes <= cap:
            raise ValueError(f"{label}.bytes exceeds hard cap or is invalid")
        for path_field in ("source_path", "vendored_path"):
            candidate = Path(str(record.get(path_field, "")))
            if candidate.is_absolute() or ".." in candidate.parts:
                raise ValueError(f"{label}.{path_field} is not a safe relative path")


def download(url: str, expected_bytes: int, maximum_bytes: int) -> bytes:
    if not 0 < expected_bytes <= maximum_bytes:
        raise ValueError(f"download size exceeds hard cap {maximum_bytes}: {expected_bytes}")
    request = urllib.request.Request(url, headers={"User-Agent": "telegram-cli-pin/1"})
    with urllib.request.urlopen(request, timeout=TIMEOUT_SECONDS) as response:
        payload = response.read(maximum_bytes + 1)
    if len(payload) > maximum_bytes:
        raise ValueError(f"download exceeded hard cap {maximum_bytes}: {url}")
    if len(payload) != expected_bytes:
        raise ValueError(
            f"unexpected payload size for {url}: expected {expected_bytes}, "
            f"received {len(payload)}"
        )
    return payload


def verify(payload: bytes, record: dict[str, Any], label: str) -> None:
    digest = hashlib.sha256(payload).hexdigest()
    if digest != record["sha256"]:
        raise ValueError(
            f"{label} sha256 mismatch: expected {record['sha256']}, received {digest}"
        )


def fetch_verified_payloads(
    manifest: dict[str, Any], *, downloader: Downloader = download
) -> dict[str, bytes]:
    validate_vendor_manifest(manifest)
    commit = manifest["upstream"]["commit"]
    payloads: dict[str, bytes] = {}
    for label in ("cmake", "schema", "license"):
        record = manifest[label]
        url = f"{RAW_BASE}/{commit}/{record['source_path']}"
        payload = downloader(url, record["bytes"], PAYLOAD_CAPS[label])
        verify(payload, record, label)
        payloads[label] = payload

    declaration = manifest["cmake"]["version_declaration"].encode("utf-8")
    if declaration not in payloads["cmake"].splitlines():
        raise ValueError("pinned TDLib version declaration not found in CMakeLists.txt")
    return payloads


def atomic_write(path: Path, payload: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary_path: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as temporary:
            temporary_path = Path(temporary.name)
            temporary.write(payload)
            temporary.flush()
            os.fsync(temporary.fileno())
        temporary_path.chmod(0o644)
        temporary_path.replace(path)
    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)


def publish_verified_payloads(
    manifest: dict[str, Any], payloads: dict[str, bytes]
) -> None:
    if set(payloads) != {"cmake", "schema", "license"}:
        raise ValueError("refusing to publish incomplete TDLib provenance bundle")
    for label in ("cmake", "schema", "license"):
        destination = (ROOT / manifest[label]["vendored_path"]).resolve()
        if destination == ROOT or ROOT not in destination.parents:
            raise ValueError(f"{label}.vendored_path escapes repository")
        atomic_write(destination, payloads[label])


def main() -> int:
    manifest = json.loads(
        read_bounded(MANIFEST_PATH, MAX_MANIFEST_BYTES, "manifest").decode("utf-8")
    )
    payloads = fetch_verified_payloads(manifest)
    publish_verified_payloads(manifest, payloads)
    upstream = manifest["upstream"]
    print(
        "vendored TDLib bundle: ok "
        f"(version={upstream['version']}, commit={upstream['commit']}, "
        "network_requests=3, publish=per-file-atomic-after-full-verify)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
