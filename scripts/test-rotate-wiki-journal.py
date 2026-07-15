#!/usr/bin/env python3
"""Negative controls for immutable journal rotation links and repair."""

from __future__ import annotations

import hashlib
import importlib.util
import os
import select
import stat
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "scripts/rotate-wiki-journal.py"


def load_rotation_module():
    spec = importlib.util.spec_from_file_location("rotate_wiki_journal", MODULE_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load rotation module")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def write_targets(root: Path) -> None:
    for relative in [
        ".memory/raw/evidence.md",
        ".memory/raw/evidence file.md",
        ".memory/decisions/decisions.md",
        ".memory/problems/problems.md",
    ]:
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("# Evidence\n", encoding="utf-8")


def entry(identifier: str, filler: str = "") -> str:
    return f"""## [2026-07-15] work | {identifier} | Test entry

- Evidence: [raw](../raw/evidence.md).
- Titled evidence: [raw](../raw/evidence.md "evidence").
- Spaced evidence: [raw](<../raw/evidence file.md>).
- Reference evidence: [raw][evidence].
[evidence]: ../raw/evidence.md "evidence"
- Digest path: `../raw/evidence.md`.
- Decision: [D](../decisions/decisions.md).
- Problem: [P](../problems/problems.md).
- Detail: {filler}
"""


def test_rotation_rebases_relative_links(rotation) -> None:
    with tempfile.TemporaryDirectory(prefix="wiki-rotation-") as directory:
        root = Path(directory)
        write_targets(root)
        active = root / ".memory/logs/work.md"
        active.parent.mkdir(parents=True)
        active.write_text(
            "# Work Journal\n\n"
            + "\n".join(
                [
                    entry("W-20260715-001", "a" * 180),
                    entry("W-20260715-002", "b" * 180),
                    entry("W-20260715-003", "c" * 180),
                ]
            ),
            encoding="utf-8",
        )

        original_cap = rotation.MAX_CHARACTERS
        rotation.MAX_CHARACTERS = 700
        try:
            rotation.rotate(root, "work")
            rotation.validate(root, "work")
        finally:
            rotation.MAX_CHARACTERS = original_cap

        shards = sorted((active.parent / "archive").glob("*.md"))
        shards = [path for path in shards if path.name != "index.md"]
        if len(shards) != 1:
            raise AssertionError(f"expected one shard, got {len(shards)}")
        payload = shards[0].read_text(encoding="utf-8")
        for target in [
            "../../raw/evidence.md",
            "../../decisions/decisions.md",
            "../../problems/problems.md",
        ]:
            if target not in payload:
                raise AssertionError(f"rotated link wasn't rebased: {target}")
        if "`../../raw/evidence.md`" not in payload:
            raise AssertionError("rotated inline evidence path wasn't rebased")
        if "[raw](<../../raw/evidence file.md>)" not in payload:
            raise AssertionError("rotated angle-bracket link wasn't rebased")
        if "[evidence]: ../../raw/evidence.md \"evidence\"" not in payload:
            raise AssertionError("rotated reference definition wasn't rebased")
        (root / ".memory/raw/evidence.md").unlink()
        try:
            rotation.validate(root, "work")
        except SystemExit as error:
            if "broken or escaping local path" not in str(error):
                raise
        else:
            raise AssertionError("archive validation accepted a broken evidence target")


def test_rotation_rejects_broken_targets_before_publication(rotation) -> None:
    with tempfile.TemporaryDirectory(prefix="wiki-broken-target-") as directory:
        root = Path(directory)
        write_targets(root)
        active = root / ".memory/logs/work.md"
        archive_root = active.parent / "archive"
        archive_root.mkdir(parents=True)
        index = archive_root / "index.md"
        index.write_text(
            "# Work Journal Archive\n\n"
            "Immutable checksum-indexed shards, oldest first.\n",
            encoding="utf-8",
        )
        active.write_text(
            "# Work Journal\n\n"
            + entry("W-20260715-001", "a" * 180)
            + entry("W-20260715-002", "b" * 500),
            encoding="utf-8",
        )
        (root / ".memory/raw/evidence.md").unlink()
        before = (active.read_bytes(), index.read_bytes())

        original_cap = rotation.MAX_CHARACTERS
        rotation.MAX_CHARACTERS = 700
        try:
            try:
                rotation.rotate(root, "work")
            except SystemExit as error:
                if "broken or escaping local path" not in str(error):
                    raise
            else:
                raise AssertionError("rotation published a shard with a broken target")
        finally:
            rotation.MAX_CHARACTERS = original_cap

        if (active.read_bytes(), index.read_bytes()) != before:
            raise AssertionError("failed rotation changed active journal or index")
        shards = [path for path in archive_root.glob("*.md") if path != index]
        if shards:
            raise AssertionError("failed rotation published an archive shard")


def git(root: Path, *arguments: str) -> None:
    subprocess.run(
        ["git", *arguments],
        cwd=root,
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        timeout=5,
    )


def create_repair_fixture(root: Path):
    write_targets(root)
    active = root / ".memory/logs/work.md"
    archive_root = active.parent / "archive"
    archive_root.mkdir(parents=True)
    active.write_text("# Work Journal\n\n" + entry("W-20260715-002"), encoding="utf-8")
    index = archive_root / "index.md"
    index.write_text(
        "# Work Journal Archive\n\n"
        "Immutable checksum-indexed shards, oldest first.\n",
        encoding="utf-8",
    )
    git(root, "init", "-q")
    git(root, "config", "user.name", "Rotation Test")
    git(root, "config", "user.email", "rotation@example.invalid")
    git(root, "add", ".")
    git(root, "-c", "commit.gpgsign=false", "commit", "-qm", "baseline")

    name = "2026-07-15--2026-07-15-001.md"
    shard = archive_root / name
    shard.write_text(
        "# Work Journal Archive\n\n"
        "Immutable rotated shard. Do not edit after creation.\n\n"
        + entry("W-20260715-001"),
        encoding="utf-8",
    )
    digest = hashlib.sha256(shard.read_bytes()).hexdigest()
    with index.open("a", encoding="utf-8") as stream:
        stream.write(
            f"\n- [2026-07-15 - 2026-07-15]({name})"
            f" — sha256 `{digest}`; entries 1\n"
        )
    return active, archive_root, index, shard


def repair_plan(rotation, root: Path, active: Path, index: Path, shard: Path):
    corrected_text = rotation.repair_relative_paths_by_resolution(
        root, shard.read_text(encoding="utf-8"), active, shard
    )
    corrected_shard = corrected_text.encode("utf-8")
    index_text = index.read_text(encoding="utf-8")
    match = list(rotation.INDEX_ENTRY.finditer(index_text))[-1]
    corrected_index = rotation.replace_last_index_digest(
        index_text, match, hashlib.sha256(corrected_shard).hexdigest()
    ).encode("utf-8")
    return corrected_shard, corrected_index


def test_repair_is_limited_to_uncommitted_last_shard(rotation) -> None:
    with tempfile.TemporaryDirectory(prefix="wiki-repair-") as directory:
        root = Path(directory)
        _active, _archive_root, index, shard = create_repair_fixture(root)
        shard.chmod(0o640)
        index.chmod(0o604)
        original_modes = (
            stat.S_IMODE(shard.stat().st_mode),
            stat.S_IMODE(index.stat().st_mode),
        )

        rotation.repair_latest_uncommitted_links(root, "work")
        rotation.validate(root, "work")
        repaired_shard = shard.read_bytes()
        repaired_index = index.read_bytes()
        rotation.repair_latest_uncommitted_links(root, "work")
        if shard.read_bytes() != repaired_shard or index.read_bytes() != repaired_index:
            raise AssertionError("repeated repair changed an already canonical shard")
        payload = shard.read_text(encoding="utf-8")
        if "../../raw/evidence.md" not in payload:
            raise AssertionError("repair didn't rebase the archived link")
        if "`../../raw/evidence.md`" not in payload:
            raise AssertionError("repair didn't rebase the inline evidence path")
        row = rotation.INDEX_ENTRY.findall(index.read_text(encoding="utf-8"))[-1]
        if row[3] != hashlib.sha256(shard.read_bytes()).hexdigest():
            raise AssertionError("repair didn't bind the new shard checksum")
        repaired_modes = (
            stat.S_IMODE(shard.stat().st_mode),
            stat.S_IMODE(index.stat().st_mode),
        )
        if repaired_modes != original_modes:
            raise AssertionError(
                f"repair changed shard/index permissions: {repaired_modes!r}"
            )

        git(root, "add", ".")
        git(root, "-c", "commit.gpgsign=false", "commit", "-qm", "archive")
        try:
            rotation.repair_latest_uncommitted_links(root, "work")
        except SystemExit as error:
            if "tracked immutable archive shard" not in str(error):
                raise
        else:
            raise AssertionError("repair accepted a committed immutable shard")


def test_repair_recovers_bounded_persisted_crash_states(rotation) -> None:
    for state in ["shard_temp", "both_temps", "shard_replaced"]:
        with tempfile.TemporaryDirectory(prefix=f"wiki-{state}-") as directory:
            root = Path(directory)
            active, archive_root, index, shard = create_repair_fixture(root)
            corrected_shard, corrected_index = repair_plan(
                rotation, root, active, index, shard
            )
            shard.chmod(0o640)
            index.chmod(0o604)
            target_modes = (0o640, 0o604)
            shard_temp = archive_root / rotation.REPAIR_SHARD_TEMP
            index_temp = archive_root / rotation.REPAIR_INDEX_TEMP
            shard_temp_mode = target_modes[0] if state == "shard_replaced" else 0o600
            rotation.write_exclusive_temp(
                shard_temp, corrected_shard, shard_temp_mode
            )
            if state != "shard_temp":
                rotation.write_exclusive_temp(index_temp, corrected_index, 0o600)
            if state == "shard_replaced":
                os.replace(shard_temp, shard)

            rotation.repair_latest_uncommitted_links(root, "work")
            rotation.validate(root, "work")
            if shard.read_bytes() != corrected_shard or index.read_bytes() != corrected_index:
                raise AssertionError(f"repair didn't recover {state}")
            if shard_temp.exists() or index_temp.exists():
                raise AssertionError(f"repair leaked temp state after {state}")
            actual_modes = (
                stat.S_IMODE(shard.stat().st_mode),
                stat.S_IMODE(index.stat().st_mode),
            )
            if actual_modes != target_modes:
                raise AssertionError(
                    f"repair changed permissions after {state}: {actual_modes!r}"
                )

    for partial in ["shard", "index"]:
        with tempfile.TemporaryDirectory(prefix=f"wiki-partial-{partial}-") as directory:
            root = Path(directory)
            active, archive_root, index, shard = create_repair_fixture(root)
            corrected_shard, corrected_index = repair_plan(
                rotation, root, active, index, shard
            )
            temp = archive_root / (
                rotation.REPAIR_SHARD_TEMP
                if partial == "shard"
                else rotation.REPAIR_INDEX_TEMP
            )
            expected = corrected_shard if partial == "shard" else corrected_index
            temp.write_bytes(expected[:7])
            rotation.repair_latest_uncommitted_links(root, "work")
            rotation.validate(root, "work")
            if temp.exists() or shard.read_bytes() != corrected_shard:
                raise AssertionError(f"repair didn't recover partial {partial} temp")


def test_stale_temp_recovery_rejects_identity_drift(rotation) -> None:
    with tempfile.TemporaryDirectory(prefix="wiki-temp-identity-") as directory:
        temp = Path(directory) / rotation.REPAIR_SHARD_TEMP
        temp.write_bytes(b"partial")
        _payload, identity = rotation.read_bounded_regular_with_identity(
            temp, rotation.MAX_JOURNAL_BYTES, "test temp"
        )
        replacement = temp.with_suffix(".replacement")
        replacement.write_bytes(b"replacement")
        os.replace(replacement, temp)
        try:
            rotation.unlink_verified_stale_temp(temp, identity, "test temp")
        except SystemExit as error:
            if "changed before stale-temp recovery" not in str(error):
                raise
        else:
            raise AssertionError("stale-temp recovery accepted inode drift")
        if temp.read_bytes() != b"replacement":
            raise AssertionError("rejected stale-temp recovery changed replacement")


def test_repair_rejects_fifo_temp_without_blocking(rotation) -> None:
    with tempfile.TemporaryDirectory(prefix="wiki-fifo-temp-") as directory:
        root = Path(directory)
        _active, archive_root, index, shard = create_repair_fixture(root)
        fifo = archive_root / rotation.REPAIR_SHARD_TEMP
        os.mkfifo(fifo, 0o600)
        before = (index.read_bytes(), shard.read_bytes())

        completed = subprocess.run(
            [
                sys.executable,
                str(MODULE_PATH),
                "--root",
                str(root),
                "--kind",
                "work",
                "--repair-latest-uncommitted-links",
            ],
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            text=True,
            timeout=2,
        )
        if completed.returncode == 0:
            raise AssertionError("repair accepted a FIFO at its deterministic temp path")
        if "expected a regular file" not in completed.stderr:
            raise AssertionError(f"unexpected FIFO rejection: {completed.stderr!r}")
        if (index.read_bytes(), shard.read_bytes()) != before:
            raise AssertionError("FIFO temp rejection changed immutable evidence")
        if not stat.S_ISFIFO(fifo.lstat().st_mode):
            raise AssertionError("FIFO temp rejection replaced the hostile path")


def test_repair_persists_shard_rename_before_index_rename(rotation) -> None:
    with tempfile.TemporaryDirectory(prefix="wiki-repair-order-") as directory:
        root = Path(directory)
        _active, _archive_root, index, shard = create_repair_fixture(root)
        events: list[str] = []
        original_replace = rotation.os.replace
        original_fsync = rotation.os.fsync

        def recording_replace(source, destination, *args, **kwargs):
            destination_path = Path(destination)
            if destination_path == shard:
                events.append("replace:shard")
            elif destination_path == index:
                events.append("replace:index")
            return original_replace(source, destination, *args, **kwargs)

        def recording_fsync(descriptor):
            if stat.S_ISDIR(os.fstat(descriptor).st_mode):
                events.append("fsync:directory")
            return original_fsync(descriptor)

        rotation.os.replace = recording_replace
        rotation.os.fsync = recording_fsync
        try:
            rotation.repair_latest_uncommitted_links(root, "work")
        finally:
            rotation.os.replace = original_replace
            rotation.os.fsync = original_fsync

        expected = [
            "replace:shard",
            "fsync:directory",
            "replace:index",
            "fsync:directory",
        ]
        if events != expected:
            raise AssertionError(f"unsafe repair persistence order: {events}")


def test_live_repair_lease_preserves_concurrent_temp(rotation) -> None:
    with tempfile.TemporaryDirectory(prefix="wiki-live-lease-") as directory:
        root = Path(directory)
        _active, archive_root, _index, _shard = create_repair_fixture(root)
        holder = subprocess.Popen(
            [
                sys.executable,
                "-c",
                (
                    "import fcntl, os, sys; "
                    "fd=os.open(sys.argv[1], os.O_RDONLY); "
                    "fcntl.flock(fd, fcntl.LOCK_EX); "
                    "print('ready', flush=True); sys.stdin.read(1)"
                ),
                str(archive_root),
            ],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        try:
            ready, _, _ = select.select([holder.stdout], [], [], 5)
            if not ready or holder.stdout is None or holder.stdout.readline().strip() != "ready":
                raise AssertionError("repair lease holder didn't become ready")
            partial = archive_root / rotation.REPAIR_SHARD_TEMP
            partial.write_bytes(b"live-partial")
            try:
                rotation.repair_latest_uncommitted_links(root, "work")
            except SystemExit as error:
                if "repair lease is busy" not in str(error):
                    raise
            else:
                raise AssertionError("concurrent repair acquired a live lease")
            if partial.read_bytes() != b"live-partial":
                raise AssertionError("concurrent repair touched a live temp")
        finally:
            if holder.stdin is not None:
                holder.stdin.write("x")
                holder.stdin.close()
            try:
                holder.wait(timeout=5)
            except subprocess.TimeoutExpired:
                holder.kill()
                holder.wait(timeout=5)
        rotation.repair_latest_uncommitted_links(root, "work")
        if (archive_root / rotation.REPAIR_SHARD_TEMP).exists():
            raise AssertionError("repair didn't clean the recovered live temp")


def test_repair_rejects_unproven_drift_paths_and_oversized_inputs(rotation) -> None:
    with tempfile.TemporaryDirectory(prefix="wiki-tamper-") as directory:
        root = Path(directory)
        active, _archive_root, index, shard = create_repair_fixture(root)
        corrected_shard, _corrected_index = repair_plan(
            rotation, root, active, index, shard
        )
        tampered = corrected_shard.replace(b"- Detail: \n", b"- Detail: tampered\n", 1)
        shard.write_bytes(tampered)
        original_index = index.read_bytes()
        try:
            rotation.repair_latest_uncommitted_links(root, "work")
        except SystemExit as error:
            if "unproven shard content drift" not in str(error):
                raise
        else:
            raise AssertionError("repair accepted non-link shard tampering")
        if shard.read_bytes() != tampered or index.read_bytes() != original_index:
            raise AssertionError("rejected tampering changed immutable evidence")

    for oversized in ["index", "shard"]:
        with tempfile.TemporaryDirectory(prefix=f"wiki-oversized-{oversized}-") as directory:
            root = Path(directory)
            _active, _archive_root, index, shard = create_repair_fixture(root)
            path = index if oversized == "index" else shard
            limit = (
                rotation.MAX_ARCHIVE_INDEX_BYTES
                if oversized == "index"
                else rotation.MAX_JOURNAL_BYTES
            )
            with path.open("wb") as stream:
                stream.truncate(limit + 1)
            try:
                rotation.repair_latest_uncommitted_links(root, "work")
            except SystemExit as error:
                if "exceeds" not in str(error):
                    raise
            else:
                raise AssertionError(f"repair accepted oversized {oversized}")

    with tempfile.TemporaryDirectory(prefix="wiki-traversal-") as directory:
        root = Path(directory)
        _active, _archive_root, index, shard = create_repair_fixture(root)
        evil = root / ".memory/raw/evil.md"
        evil.write_bytes(shard.read_bytes())
        index_text = index.read_text(encoding="utf-8").replace(
            shard.name, "../../raw/evil.md"
        )
        index.write_text(index_text, encoding="utf-8")
        before = (index.read_bytes(), evil.read_bytes(), shard.read_bytes())
        try:
            rotation.repair_latest_uncommitted_links(root, "work")
        except SystemExit as error:
            if "non-canonical archive shard basename" not in str(error):
                raise
        else:
            raise AssertionError("repair accepted an archive path traversal")
        after = (index.read_bytes(), evil.read_bytes(), shard.read_bytes())
        if after != before:
            raise AssertionError("path traversal rejection changed files")


def test_repair_rejects_historically_tracked_shard(rotation) -> None:
    with tempfile.TemporaryDirectory(prefix="wiki-head-shard-") as directory:
        root = Path(directory)
        _active, archive_root, index, shard = create_repair_fixture(root)
        git(root, "add", "--", shard.relative_to(root).as_posix())
        git(root, "-c", "commit.gpgsign=false", "commit", "-qm", "orphan shard")
        git(root, "rm", "--cached", "--", shard.relative_to(root).as_posix())
        before = (index.read_bytes(), shard.read_bytes())

        try:
            rotation.repair_latest_uncommitted_links(root, "work")
        except SystemExit as error:
            if "historically tracked immutable archive shard" not in str(error):
                raise
        else:
            raise AssertionError("repair accepted a shard present in HEAD")

        if (index.read_bytes(), shard.read_bytes()) != before:
            raise AssertionError("HEAD shard rejection changed immutable evidence")
        if any(archive_root.glob(".repair-links*")):
            raise AssertionError("HEAD shard rejection created repair temps")


def test_repair_requires_exact_committed_index_prefix(rotation) -> None:
    with tempfile.TemporaryDirectory(prefix="wiki-index-prefix-") as directory:
        root = Path(directory)
        _active, archive_root, index, shard = create_repair_fixture(root)
        row = list(rotation.INDEX_ENTRY.finditer(index.read_text(encoding="utf-8")))[
            -1
        ].group(0)
        committed_with_trailing_drift = (
            "# Work Journal Archive\n\n"
            "Immutable checksum-indexed shards, oldest first. \n\n"
        )
        index.write_text(committed_with_trailing_drift, encoding="utf-8")
        git(root, "add", "--", index.relative_to(root).as_posix())
        git(
            root,
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            "noncanonical index suffix",
        )
        index.write_text(
            committed_with_trailing_drift.rstrip() + "\n\n" + row + "\n",
            encoding="utf-8",
        )
        before = (index.read_bytes(), shard.read_bytes())

        try:
            rotation.repair_latest_uncommitted_links(root, "work")
        except SystemExit as error:
            if "committed archive index suffix isn't canonical" not in str(error):
                raise
        else:
            raise AssertionError("repair normalized unrelated committed index suffix")

        if (index.read_bytes(), shard.read_bytes()) != before:
            raise AssertionError("index-prefix rejection changed immutable evidence")
        if any(archive_root.glob(".repair-links*")):
            raise AssertionError("index-prefix rejection created repair temps")


def test_repair_cli_requires_one_explicit_kind(rotation) -> None:
    del rotation
    with tempfile.TemporaryDirectory(prefix="wiki-repair-cli-") as directory:
        root = Path(directory)
        _active, _archive_root, index, shard = create_repair_fixture(root)
        before = (index.read_bytes(), shard.read_bytes())
        for arguments in [[], ["--kind", "all"], ["--all"]]:
            completed = subprocess.run(
                [
                    sys.executable,
                    str(MODULE_PATH),
                    "--root",
                    str(root),
                    "--repair-latest-uncommitted-links",
                    *arguments,
                ],
                check=False,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                timeout=5,
            )
            if completed.returncode == 0:
                raise AssertionError(f"repair accepted non-specific scope: {arguments}")
            if (index.read_bytes(), shard.read_bytes()) != before:
                raise AssertionError("rejected repair scope changed archive evidence")


def test_rotation_rejects_unsupported_inline_link_syntax(rotation) -> None:
    with tempfile.TemporaryDirectory(prefix="wiki-link-syntax-") as directory:
        root = Path(directory)
        write_targets(root)
        active = root / ".memory/logs/work.md"
        active.parent.mkdir(parents=True)
        active.write_text(
            "# Work Journal\n\n"
            + entry("W-20260715-001", "[raw](../raw/evidence(foo).md)")
            + entry("W-20260715-002", "x" * 500),
            encoding="utf-8",
        )
        original_cap = rotation.MAX_CHARACTERS
        rotation.MAX_CHARACTERS = 700
        try:
            try:
                rotation.rotate(root, "work")
            except SystemExit as error:
                if "unsupported inline Markdown link syntax" not in str(error):
                    raise
            else:
                raise AssertionError("rotation accepted unsupported local link syntax")
        finally:
            rotation.MAX_CHARACTERS = original_cap
        if (active.parent / "archive").exists():
            shards = list((active.parent / "archive").glob("*.md"))
            if shards:
                raise AssertionError("failed rotation published an archive shard")


def main() -> None:
    rotation = load_rotation_module()
    test_rotation_rebases_relative_links(rotation)
    test_rotation_rejects_broken_targets_before_publication(rotation)
    test_repair_is_limited_to_uncommitted_last_shard(rotation)
    test_repair_recovers_bounded_persisted_crash_states(rotation)
    test_stale_temp_recovery_rejects_identity_drift(rotation)
    test_repair_rejects_fifo_temp_without_blocking(rotation)
    test_repair_persists_shard_rename_before_index_rename(rotation)
    test_live_repair_lease_preserves_concurrent_temp(rotation)
    test_repair_rejects_unproven_drift_paths_and_oversized_inputs(rotation)
    test_repair_rejects_historically_tracked_shard(rotation)
    test_repair_requires_exact_committed_index_prefix(rotation)
    test_repair_cli_requires_one_explicit_kind(rotation)
    test_rotation_rejects_unsupported_inline_link_syntax(rotation)
    print("wiki rotation tests: ok")


if __name__ == "__main__":
    main()
