#!/usr/bin/env python3
"""Rotate and validate separate Karpathy Wiki journals."""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import os
import posixpath
import re
import stat
import subprocess
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import unquote, urlsplit, urlunsplit


ENTRY = re.compile(r"(?m)^## \[(\d{4}-\d{2}-\d{2})\] [^\n]+$")
INDEX_ENTRY = re.compile(
    r"(?m)^- \[(\d{4}-\d{2}-\d{2}) - (\d{4}-\d{2}-\d{2})\]"
    r"\(([^)]+\.md)\) — sha256 `([0-9a-f]{64})`; entries (\d+)$"
)
MARKDOWN_LINK = re.compile(
    r"(?P<prefix>!?\[[^\]\n]*\]\([ \t]*)"
    r"(?P<target><[^>\n]+>|[^()\s]+)"
    r"(?P<suffix>[ \t]*(?:(?:\"[^\"\n]*\"|'[^'\n]*'|\([^\)\n]*\))[ \t]*)?\))"
)
REFERENCE_DEFINITION = re.compile(
    r"(?m)^(?P<prefix>\[[^\]\n]+\]:[ \t]*)"
    r"(?P<target><[^>\n]+>|[^ \t\n]+)"
    r"(?P<suffix>[ \t]*(?:(?:\"[^\"\n]*\"|'[^'\n]*'|\([^\)\n]*\))[ \t]*)?)$"
)
MARKDOWN_LINK_START = re.compile(r"!?\[[^\]\n]*\]\(")
INLINE_RELATIVE_PATH = re.compile(
    r"(?P<prefix>`)(?P<target>\.\.?/[^`\s]+\.md(?:[?#][^`\s]*)?)(?P<suffix>`)"
)
MAX_LINES = 1_000
MAX_CHARACTERS = 16_000
MAX_JOURNAL_BYTES = 64 * 1024
MAX_ARCHIVE_INDEX_BYTES = 1024 * 1024
MAX_ARCHIVE_SHARDS = 4_096
MAX_ENTRIES_PER_SHARD = 1_000
REPAIR_SHARD_TEMP = ".repair-links.shard.tmp"
REPAIR_INDEX_TEMP = ".repair-links.index.tmp"
SHARD_BASENAME = re.compile(
    r"\d{4}-\d{2}-\d{2}--\d{4}-\d{2}-\d{2}-\d{3}\.md"
)


@dataclass(frozen=True)
class Journal:
    active: str
    archive_title: str


JOURNALS = {
    "work": Journal(".memory/logs/work.md", "Work Journal Archive"),
    "decisions": Journal(".memory/decisions/decisions.md", "Decision Journal Archive"),
    "problems": Journal(".memory/problems/problems.md", "Problem Journal Archive"),
}


FileIdentity = tuple[int, int, int, int, int]


def file_identity(metadata: os.stat_result) -> FileIdentity:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_size,
        metadata.st_mtime_ns,
        metadata.st_mode,
    )


def read_bounded_regular_with_identity(
    path: Path, limit: int, label: str
) -> tuple[bytes, FileIdentity]:
    flags = (
        os.O_RDONLY
        | getattr(os, "O_NOFOLLOW", 0)
        | getattr(os, "O_NONBLOCK", 0)
    )
    try:
        descriptor = os.open(path, flags)
    except OSError as error:
        raise SystemExit(f"{label}: expected a readable regular file") from error
    try:
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode):
            raise SystemExit(f"{label}: expected a regular file")
        if before.st_size > limit:
            raise SystemExit(f"{label}: exceeds the {limit}-byte cap")
        chunks: list[bytes] = []
        size = 0
        while True:
            chunk = os.read(descriptor, min(64 * 1024, limit + 1 - size))
            if not chunk:
                break
            chunks.append(chunk)
            size += len(chunk)
            if size > limit:
                raise SystemExit(f"{label}: exceeds the {limit}-byte cap")
        after = os.fstat(descriptor)
        identity = file_identity(before)
        if identity != file_identity(after):
            raise SystemExit(f"{label}: changed while being read")
        return b"".join(chunks), identity
    finally:
        os.close(descriptor)


def read_bounded_regular(path: Path, limit: int, label: str) -> bytes:
    return read_bounded_regular_with_identity(path, limit, label)[0]


def decode_utf8(payload: bytes, label: str) -> str:
    try:
        return payload.decode("utf-8")
    except UnicodeDecodeError as error:
        raise SystemExit(f"{label}: isn't UTF-8") from error


def read_bounded_text(path: Path, limit: int, label: str) -> str:
    return decode_utf8(read_bounded_regular(path, limit, label), label)


def regular_permissions(identity: FileIdentity) -> int:
    return stat.S_IMODE(identity[4]) & 0o777


def checked_archive_path(archive_root: Path, name: str, label: str) -> Path:
    if not SHARD_BASENAME.fullmatch(name) or Path(name).name != name:
        raise SystemExit(f"{label}: non-canonical archive shard basename")
    path = archive_root / name
    if path.parent.resolve() != archive_root.resolve():
        raise SystemExit(f"{label}: archive shard escapes archive directory")
    return path


def parse_entries(text: str) -> tuple[str, list[str], list[str]]:
    matches = list(ENTRY.finditer(text))
    if not matches:
        raise ValueError("journal has no valid entries")
    prefix = text[: matches[0].start()].rstrip()
    entries = [
        text[match.start() : matches[index + 1].start() if index + 1 < len(matches) else len(text)].strip()
        for index, match in enumerate(matches)
    ]
    return prefix, entries, [match.group(1) for match in matches]


def fits(text: str) -> bool:
    return len(text.splitlines()) <= MAX_LINES and len(text) <= MAX_CHARACTERS


def shard_name(archive_root: Path, first: str, last: str) -> str:
    prefix = f"{first}--{last}"
    for sequence in range(1, MAX_ARCHIVE_SHARDS + 1):
        if not (archive_root / f"{prefix}-{sequence:03d}.md").exists():
            return f"{prefix}-{sequence:03d}.md"
    raise SystemExit("archive shard sequence exceeds its cap")


def local_link_parts(target: str):
    if target.startswith("<") and target.endswith(">"):
        target = target[1:-1]
    if target.startswith(("#", "/", "//")):
        return None
    parsed = urlsplit(target)
    if parsed.scheme or parsed.netloc or not parsed.path:
        return None
    return parsed


def rebase_target(target: str, source: Path, destination: Path) -> str:
    wrapped = target.startswith("<") and target.endswith(">")
    raw_target = target[1:-1] if wrapped else target
    parsed = local_link_parts(raw_target)
    if parsed is None:
        return target
    rooted = posixpath.normpath(posixpath.join(source.as_posix(), parsed.path))
    rebased = posixpath.relpath(rooted, destination.as_posix())
    value = urlunsplit(("", "", rebased, parsed.query, parsed.fragment))
    return f"<{value}>" if wrapped else value


def validate_supported_markdown_syntax(text: str) -> None:
    complete_starts = {match.start() for match in MARKDOWN_LINK.finditer(text)}
    for match in MARKDOWN_LINK_START.finditer(text):
        if match.start() not in complete_starts:
            raise SystemExit("unsupported inline Markdown link syntax in journal entry")


def rewrite_relative_targets(text: str, rewrite) -> str:
    validate_supported_markdown_syntax(text)

    def replace_markdown(match: re.Match[str]) -> str:
        target = rewrite(match.group("target"))
        return f"{match.group('prefix')}{target}{match.group('suffix')}"

    def replace_inline(match: re.Match[str]) -> str:
        target = rewrite(match.group("target"))
        return f"{match.group('prefix')}{target}{match.group('suffix')}"

    def replace_reference(match: re.Match[str]) -> str:
        target = rewrite(match.group("target"))
        return f"{match.group('prefix')}{target}{match.group('suffix')}"

    return INLINE_RELATIVE_PATH.sub(
        replace_inline,
        REFERENCE_DEFINITION.sub(
            replace_reference, MARKDOWN_LINK.sub(replace_markdown, text)
        ),
    )


def rebase_relative_markdown_links(text: str, source: Path, destination: Path) -> str:
    return rewrite_relative_targets(
        text, lambda target: rebase_target(target, source, destination)
    )


def relative_targets(text: str):
    validate_supported_markdown_syntax(text)
    for pattern in [MARKDOWN_LINK, REFERENCE_DEFINITION, INLINE_RELATIVE_PATH]:
        for match in pattern.finditer(text):
            yield match.group("target")


def target_exists(root: Path, source: Path, target: str) -> bool:
    parsed = local_link_parts(target)
    if parsed is None:
        return True
    canonical_root = root.resolve()
    candidate = (source.parent / unquote(parsed.path)).resolve()
    try:
        candidate.relative_to(canonical_root)
    except ValueError:
        return False
    return candidate.exists()


def validate_local_links(root: Path, source: Path, text: str, label: str) -> None:
    for target in relative_targets(text):
        if not target_exists(root, source, target):
            raise SystemExit(f"{label}: broken or escaping local path {target!r}")


def repair_relative_paths_by_resolution(
    root: Path,
    text: str,
    active: Path,
    archive: Path,
) -> str:
    active_relative = active.parent.relative_to(root)
    archive_relative = archive.parent.relative_to(root)

    def repair(target: str) -> str:
        if local_link_parts(target) is None:
            return target
        valid_as_active = target_exists(root, active, target)
        valid_as_archive = target_exists(root, archive, target)
        if valid_as_archive and not valid_as_active:
            return target
        if valid_as_active and not valid_as_archive:
            return rebase_target(target, active_relative, archive_relative)
        raise SystemExit("repair refuses ambiguous or broken relative path context")

    return rewrite_relative_targets(text, repair)


def rotate(root: Path, kind: str) -> None:
    journal = JOURNALS[kind]
    active = root / journal.active
    source = read_bounded_text(active, MAX_JOURNAL_BYTES, f"{kind}:active journal")
    prefix, entries, dates = parse_entries(source)
    if fits(source):
        print(f"{kind}: rotation not needed")
        return

    keep_from = len(entries) - 1
    while keep_from > 0:
        candidate = prefix + "\n\n" + "\n\n".join(entries[keep_from - 1 :]) + "\n"
        if not fits(candidate):
            break
        keep_from -= 1
    if keep_from == 0:
        raise SystemExit(f"{kind}: newest entry alone exceeds active journal limits")

    archive_root = active.parent / "archive"
    archive_root.mkdir(parents=True, exist_ok=True)
    name = shard_name(archive_root, dates[0], dates[keep_from - 1])
    archive = archive_root / name
    active_root = Path(journal.active).parent
    archive_relative = archive_root.relative_to(root)
    archived_entries = [
        rebase_relative_markdown_links(entry, active_root, archive_relative)
        for entry in entries[:keep_from]
    ]
    archive_payload = (
        f"# {journal.archive_title}\n\nImmutable rotated shard. Do not edit after creation.\n\n"
        + "\n\n".join(archived_entries)
        + "\n"
    )
    encoded_archive = archive_payload.encode("utf-8")
    if len(encoded_archive) > MAX_JOURNAL_BYTES:
        raise SystemExit(f"{kind}: archive shard exceeds its byte cap")
    validate_local_links(root, archive, archive_payload, f"{kind}:{name}")

    index = archive_root / "index.md"
    index_text = (
        read_bounded_text(
            index, MAX_ARCHIVE_INDEX_BYTES, f"{kind}:archive index"
        ).rstrip()
        if index.exists()
        else f"# {journal.archive_title}\n\nImmutable checksum-indexed shards, oldest first."
    )
    if len(INDEX_ENTRY.findall(index_text)) >= MAX_ARCHIVE_SHARDS:
        raise SystemExit(f"{kind}: archive index exceeds its row cap")
    digest = hashlib.sha256(encoded_archive).hexdigest()
    corrected_index = (
        index_text
        + f"\n\n- [{dates[0]} - {dates[keep_from - 1]}]({name})"
        + f" — sha256 `{digest}`; entries {len(archived_entries)}\n"
    )
    if len(corrected_index.encode("utf-8")) > MAX_ARCHIVE_INDEX_BYTES:
        raise SystemExit(f"{kind}: archive index exceeds its byte cap")

    archive.write_bytes(encoded_archive)
    active.write_text(
        prefix + "\n\n" + "\n\n".join(entries[keep_from:]) + "\n",
        encoding="utf-8",
    )
    index.write_text(corrected_index, encoding="utf-8")
    print(f"{kind}: rotated {len(archived_entries)} entries to {archive.relative_to(root)}")


def git_small_output(root: Path, *arguments: str, limit: int = 256) -> bytes:
    completed = subprocess.run(
        ["git", *arguments],
        cwd=root,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=5,
    )
    if completed.returncode != 0:
        raise SystemExit(
            f"git {' '.join(arguments)} failed while proving uncommitted repair scope"
        )
    if len(completed.stdout) > limit:
        raise SystemExit(f"git {' '.join(arguments)} exceeded its output cap")
    return completed.stdout


def head_commit(root: Path) -> str:
    commit = git_small_output(root, "rev-parse", "--verify", "HEAD", limit=128)
    commit_id = commit.decode("ascii").strip()
    if not re.fullmatch(r"[0-9a-f]{40,64}", commit_id):
        raise SystemExit("git HEAD identity isn't canonical")
    return commit_id


def committed_file(root: Path, relative: str, limit: int) -> bytes:
    commit_id = head_commit(root)
    object_name = f"{commit_id}:{relative}"
    size_payload = git_small_output(root, "cat-file", "-s", object_name, limit=64)
    try:
        size = int(size_payload)
    except ValueError as error:
        raise SystemExit("git committed file size isn't numeric") from error
    if size > limit:
        raise SystemExit(f"committed {relative} exceeds the {limit}-byte cap")
    payload = git_small_output(root, "show", object_name, limit=limit)
    if len(payload) != size:
        raise SystemExit(f"committed {relative} size changed during proof")
    return payload


def prove_uncommitted_last_shard(
    root: Path,
    archive: Path,
    index: Path,
    index_text: str,
    row: str,
) -> None:
    archive_relative = archive.relative_to(root).as_posix()
    tracked = subprocess.run(
        ["git", "ls-files", "--error-unmatch", "--", archive_relative],
        cwd=root,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        timeout=5,
    )
    if tracked.returncode == 0:
        raise SystemExit("repair refuses a tracked immutable archive shard")
    if tracked.returncode != 1:
        raise SystemExit("cannot prove that archive shard is untracked")

    committed_shard = git_small_output(
        root,
        "ls-tree",
        "--name-only",
        "-z",
        head_commit(root),
        "--",
        archive_relative,
        limit=len(archive_relative.encode("utf-8")) + 1,
    )
    if committed_shard:
        raise SystemExit("repair refuses a historically tracked immutable archive shard")

    index_relative = index.relative_to(root).as_posix()
    try:
        committed = committed_file(
            root, index_relative, MAX_ARCHIVE_INDEX_BYTES
        ).decode("utf-8")
    except UnicodeDecodeError as error:
        raise SystemExit("committed archive index isn't UTF-8") from error
    if committed != committed.rstrip() + "\n":
        raise SystemExit("committed archive index suffix isn't canonical")
    expected = committed + "\n" + row + "\n"
    if index_text != expected:
        raise SystemExit("repair refuses index changes beyond one uncommitted final row")


def write_exclusive_temp(path: Path, payload: bytes, mode: int = 0o600) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        with os.fdopen(descriptor, "wb", closefd=True) as stream:
            os.fchmod(stream.fileno(), mode & 0o777)
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
    except BaseException:
        path.unlink(missing_ok=True)
        raise


def unlink_verified_stale_temp(
    path: Path, expected_identity: FileIdentity, label: str
) -> None:
    try:
        current = path.lstat()
    except OSError as error:
        raise SystemExit(f"{label}: changed before stale-temp recovery") from error
    if file_identity(current) != expected_identity or not stat.S_ISREG(current.st_mode):
        raise SystemExit(f"{label}: changed before stale-temp recovery")
    path.unlink()


def prepare_reusable_temp(path: Path, payload: bytes, label: str, mode: int) -> None:
    try:
        write_exclusive_temp(path, payload, mode)
    except FileExistsError:
        limit = max(MAX_JOURNAL_BYTES, MAX_ARCHIVE_INDEX_BYTES)
        existing, identity = read_bounded_regular_with_identity(path, limit, label)
        if existing == payload and regular_permissions(identity) == mode:
            return
        unlink_verified_stale_temp(path, identity, label)
        write_exclusive_temp(path, payload, mode)


def replace_last_index_digest(index_text: str, match: re.Match[str], digest: str) -> str:
    row = match.group(0)
    previous = match.group(4)
    corrected = row.replace(f"sha256 `{previous}`", f"sha256 `{digest}`", 1)
    return index_text[: match.start()] + corrected + index_text[match.end() :]


@contextmanager
def exclusive_repair_lease(archive_root: Path):
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(archive_root, flags)
    except OSError as error:
        raise SystemExit("repair archive root must be a real directory") from error
    try:
        metadata = os.fstat(descriptor)
        if not stat.S_ISDIR(metadata.st_mode):
            raise SystemExit("repair archive root must be a directory")
        try:
            fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise SystemExit("repair lease is busy") from error
        try:
            yield descriptor
        finally:
            fcntl.flock(descriptor, fcntl.LOCK_UN)
    finally:
        os.close(descriptor)


def repair_latest_uncommitted_links(root: Path, kind: str) -> None:
    archive_root = (root / JOURNALS[kind].active).parent / "archive"
    with exclusive_repair_lease(archive_root) as lease_descriptor:
        repair_latest_uncommitted_links_under_lease(root, kind, lease_descriptor)


def repair_latest_uncommitted_links_under_lease(
    root: Path, kind: str, lease_descriptor: int
) -> None:
    journal = JOURNALS[kind]
    active = root / journal.active
    archive_root = active.parent / "archive"
    index = archive_root / "index.md"
    index_payload, index_identity = read_bounded_regular_with_identity(
        index, MAX_ARCHIVE_INDEX_BYTES, f"{kind}:archive index"
    )
    index_text = decode_utf8(index_payload, f"{kind}:archive index")
    matches = list(INDEX_ENTRY.finditer(index_text))
    if len(matches) > MAX_ARCHIVE_SHARDS:
        raise SystemExit(f"{kind}: archive index exceeds its row cap")
    if not matches:
        raise SystemExit(f"{kind}: repair needs an indexed archive shard")
    match = matches[-1]
    if match.end() != len(index_text.rstrip()):
        raise SystemExit(f"{kind}: final archive index row isn't canonical")

    archive = checked_archive_path(archive_root, match.group(3), f"{kind}:repair")
    payload, archive_identity = read_bounded_regular_with_identity(
        archive, MAX_JOURNAL_BYTES, f"{kind}:{archive.name}"
    )
    text = payload.decode("utf-8")
    _prefix, entries, dates = parse_entries(text)
    if len(entries) > MAX_ENTRIES_PER_SHARD:
        raise SystemExit(f"{kind}: repair shard exceeds its entry cap")
    if (
        len(entries) != int(match.group(5))
        or dates[0] != match.group(1)
        or dates[-1] != match.group(2)
    ):
        raise SystemExit(f"{kind}: repair refuses entry/date drift")
    prove_uncommitted_last_shard(root, archive, index, index_text, match.group(0))

    indexed_digest = match.group(4)
    current_digest = hashlib.sha256(payload).hexdigest()
    if current_digest == indexed_digest:
        corrected_text = repair_relative_paths_by_resolution(root, text, active, archive)
        if corrected_text == text:
            print(f"{kind}: latest uncommitted shard links are already canonical")
            return
        corrected_payload = corrected_text.encode("utf-8")
    else:
        # Recover the only permitted interrupted state: shard already replaced,
        # while the uncommitted final index row still has its previous digest.
        active_relative = active.parent.relative_to(root)
        archive_relative = archive.parent.relative_to(root)
        original_text = rebase_relative_markdown_links(
            text, archive_relative, active_relative
        )
        if hashlib.sha256(original_text.encode("utf-8")).hexdigest() != indexed_digest:
            raise SystemExit(f"{kind}: repair refuses unproven shard content drift")
        if (
            rebase_relative_markdown_links(
                original_text, active_relative, archive_relative
            )
            != text
        ):
            raise SystemExit(f"{kind}: repair refuses non-canonical shard content")
        corrected_payload = payload

    corrected_digest = hashlib.sha256(corrected_payload).hexdigest()
    corrected_index = replace_last_index_digest(index_text, match, corrected_digest).encode(
        "utf-8"
    )
    shard_temp = archive_root / REPAIR_SHARD_TEMP
    index_temp = archive_root / REPAIR_INDEX_TEMP
    if corrected_payload != payload:
        prepare_reusable_temp(
            shard_temp,
            corrected_payload,
            f"{kind}:repair shard temp",
            regular_permissions(archive_identity),
        )
    prepare_reusable_temp(
        index_temp,
        corrected_index,
        f"{kind}:repair index temp",
        regular_permissions(index_identity),
    )
    if corrected_payload != payload:
        os.replace(shard_temp, archive)
        os.fsync(lease_descriptor)
    os.replace(index_temp, index)
    os.fsync(lease_descriptor)
    print(f"{kind}: repaired links in uncommitted shard {archive.relative_to(root)}")


def validate(root: Path, kind: str) -> None:
    journal = JOURNALS[kind]
    active = root / journal.active
    active_text = read_bounded_text(active, MAX_JOURNAL_BYTES, f"{kind}:active journal")
    if not fits(active_text):
        raise SystemExit(f"{kind}: active journal exceeds limits; rotate it")
    _prefix, active_entries, _dates = parse_entries(active_text)

    archive_root = active.parent / "archive"
    index = archive_root / "index.md"
    if not index.is_file():
        raise SystemExit(f"{kind}: archive index is missing")
    index_text = read_bounded_text(
        index, MAX_ARCHIVE_INDEX_BYTES, f"{kind}:archive index"
    )
    index_rows = INDEX_ENTRY.findall(index_text)
    if len(index_rows) > MAX_ARCHIVE_SHARDS:
        raise SystemExit(f"{kind}: archive index exceeds its row cap")
    indexed = [row[2] for row in index_rows]
    files = []
    with os.scandir(archive_root) as entries:
        for entry in entries:
            if entry.name != "index.md" and entry.name.endswith(".md"):
                files.append(entry.name)
                if len(files) > MAX_ARCHIVE_SHARDS:
                    raise SystemExit(f"{kind}: archive directory exceeds its shard cap")
    files.sort()
    if len(indexed) != len(set(indexed)) or set(indexed) != set(files):
        raise SystemExit(f"{kind}: archive index/file drift")

    seen: set[str] = set()
    for first, last, name, expected_digest, expected_count in index_rows:
        path = checked_archive_path(archive_root, name, f"{kind}:archive index")
        payload = read_bounded_regular(path, MAX_JOURNAL_BYTES, f"{kind}:{name}")
        text = payload.decode("utf-8")
        _archive_prefix, entries, dates = parse_entries(text)
        if len(entries) > MAX_ENTRIES_PER_SHARD:
            raise SystemExit(f"{kind}: {name} exceeds its entry cap")
        if hashlib.sha256(payload).hexdigest() != expected_digest:
            raise SystemExit(f"{kind}: checksum drift in {name}")
        if len(entries) != int(expected_count) or dates[0] != first or dates[-1] != last:
            raise SystemExit(f"{kind}: entry/date drift in {name}")
        validate_local_links(root, path, text, f"{kind}:{name}")
        for entry in entries:
            normalized = rebase_relative_markdown_links(
                entry,
                archive_root.relative_to(root),
                Path(journal.active).parent,
            )
            digest = hashlib.sha256(normalized.encode("utf-8")).hexdigest()
            if digest in seen:
                raise SystemExit(f"{kind}: duplicate archived entry")
            seen.add(digest)
    for entry in active_entries:
        digest = hashlib.sha256(entry.encode("utf-8")).hexdigest()
        if digest in seen:
            raise SystemExit(f"{kind}: active/archive duplicate entry")
        seen.add(digest)
    print(f"{kind}: contract valid")


def selected_kinds(kind: str) -> list[str]:
    return list(JOURNALS) if kind == "all" else [kind]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--kind", choices=[*JOURNALS, "all"])
    parser.add_argument("--all", action="store_true", help="alias for --kind all")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--repair-latest-uncommitted-links", action="store_true")
    args = parser.parse_args()
    if args.check and args.repair_latest_uncommitted_links:
        parser.error("--check and --repair-latest-uncommitted-links are mutually exclusive")
    if args.repair_latest_uncommitted_links and (args.all or args.kind not in JOURNALS):
        parser.error("repair requires one explicit --kind work|decisions|problems")
    root = args.root.resolve()
    kind = "all" if args.all or args.kind is None else args.kind
    for selected in selected_kinds(kind):
        if args.repair_latest_uncommitted_links:
            repair_latest_uncommitted_links(root, selected)
            validate(root, selected)
        elif args.check:
            validate(root, selected)
        else:
            rotate(root, selected)
            validate(root, selected)


if __name__ == "__main__":
    main()
