#!/usr/bin/env python3
"""Rotate and validate separate Karpathy Wiki journals."""

from __future__ import annotations

import argparse
import hashlib
import re
from dataclasses import dataclass
from pathlib import Path


ENTRY = re.compile(r"(?m)^## \[(\d{4}-\d{2}-\d{2})\] [^\n]+$")
INDEX_ENTRY = re.compile(
    r"(?m)^- \[(\d{4}-\d{2}-\d{2}) - (\d{4}-\d{2}-\d{2})\]"
    r"\(([^)]+\.md)\) — sha256 `([0-9a-f]{64})`; entries (\d+)$"
)
MAX_LINES = 1_000
MAX_CHARACTERS = 16_000


@dataclass(frozen=True)
class Journal:
    active: str
    archive_title: str


JOURNALS = {
    "work": Journal(".memory/logs/work.md", "Work Journal Archive"),
    "decisions": Journal(".memory/decisions/decisions.md", "Decision Journal Archive"),
    "problems": Journal(".memory/problems/problems.md", "Problem Journal Archive"),
}


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
    sequence = 1
    while (archive_root / f"{prefix}-{sequence:03d}.md").exists():
        sequence += 1
    return f"{prefix}-{sequence:03d}.md"


def rotate(root: Path, kind: str) -> None:
    journal = JOURNALS[kind]
    active = root / journal.active
    source = active.read_text(encoding="utf-8")
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
    archived_entries = entries[:keep_from]
    archive.write_text(
        f"# {journal.archive_title}\n\nImmutable rotated shard. Do not edit after creation.\n\n"
        + "\n\n".join(archived_entries)
        + "\n",
        encoding="utf-8",
    )
    active.write_text(prefix + "\n\n" + "\n\n".join(entries[keep_from:]) + "\n", encoding="utf-8")

    index = archive_root / "index.md"
    index_text = (
        index.read_text(encoding="utf-8").rstrip()
        if index.exists()
        else f"# {journal.archive_title}\n\nImmutable checksum-indexed shards, oldest first."
    )
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    index.write_text(
        index_text
        + f"\n\n- [{dates[0]} - {dates[keep_from - 1]}]({name})"
        + f" — sha256 `{digest}`; entries {len(archived_entries)}\n",
        encoding="utf-8",
    )
    print(f"{kind}: rotated {len(archived_entries)} entries to {archive.relative_to(root)}")


def validate(root: Path, kind: str) -> None:
    journal = JOURNALS[kind]
    active = root / journal.active
    active_text = active.read_text(encoding="utf-8")
    if not fits(active_text):
        raise SystemExit(f"{kind}: active journal exceeds limits; rotate it")
    _prefix, active_entries, _dates = parse_entries(active_text)

    archive_root = active.parent / "archive"
    index = archive_root / "index.md"
    if not index.is_file():
        raise SystemExit(f"{kind}: archive index is missing")
    index_rows = INDEX_ENTRY.findall(index.read_text(encoding="utf-8"))
    indexed = [row[2] for row in index_rows]
    files = sorted(path.name for path in archive_root.glob("*.md") if path.name != "index.md")
    if len(indexed) != len(set(indexed)) or set(indexed) != set(files):
        raise SystemExit(f"{kind}: archive index/file drift")

    seen: set[str] = set()
    for first, last, name, expected_digest, expected_count in index_rows:
        path = archive_root / name
        payload = path.read_bytes()
        _archive_prefix, entries, dates = parse_entries(payload.decode("utf-8"))
        if hashlib.sha256(payload).hexdigest() != expected_digest:
            raise SystemExit(f"{kind}: checksum drift in {name}")
        if len(entries) != int(expected_count) or dates[0] != first or dates[-1] != last:
            raise SystemExit(f"{kind}: entry/date drift in {name}")
        for entry in entries:
            digest = hashlib.sha256(entry.encode("utf-8")).hexdigest()
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
    parser.add_argument("--kind", choices=[*JOURNALS, "all"], default="all")
    parser.add_argument("--all", action="store_true", help="alias for --kind all")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    kind = "all" if args.all else args.kind
    for selected in selected_kinds(kind):
        if args.check:
            validate(root, selected)
        else:
            rotate(root, selected)
            validate(root, selected)


if __name__ == "__main__":
    main()
