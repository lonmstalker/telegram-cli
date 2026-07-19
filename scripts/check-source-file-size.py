#!/usr/bin/env python3
"""Ограничивает размер Rust source-файлов legacy-ratchet правилом."""

from __future__ import annotations

from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parent.parent
LINE_LIMIT = 1_500
RATCHET = {
    "apps/telegramd/src/server.rs": 2_542,
    "crates/telegram-core/src/workflows/mod.rs": 2_146,
}


def rust_sources(root: Path) -> dict[str, Path]:
    sources: dict[str, Path] = {}
    for source_root in (root / "crates", root / "apps"):
        if not source_root.is_dir():
            continue
        for path in source_root.rglob("*.rs"):
            if path.name.endswith("generated.rs"):
                continue
            sources[path.relative_to(root).as_posix()] = path
    return sources


def line_count(path: Path) -> int:
    with path.open(encoding="utf-8") as source:
        return sum(1 for _ in source)


def violations(
    root: Path = ROOT,
    *,
    line_limit: int = LINE_LIMIT,
    ratchet: dict[str, int] = RATCHET,
) -> list[str]:
    sources = rust_sources(root)
    errors: list[str] = []

    for relative_path, maximum_lines in sorted(ratchet.items()):
        path = sources.get(relative_path)
        if path is None:
            errors.append(
                f"{relative_path}: отсутствует; удалите запись из RATCHET"
            )
            continue

        actual_lines = line_count(path)
        if actual_lines <= line_limit:
            errors.append(
                f"{relative_path}: {actual_lines} строк уже не превышают "
                f"порог {line_limit}; удалите запись из RATCHET"
            )
        elif actual_lines > maximum_lines:
            errors.append(
                f"{relative_path}: вырос с ratchet {maximum_lines} до "
                f"{actual_lines} строк"
            )
        elif actual_lines < maximum_lines:
            errors.append(
                f"{relative_path}: уменьшился с ratchet {maximum_lines} до "
                f"{actual_lines} строк; снизьте RATCHET до текущего "
                "размера"
            )

    for relative_path, path in sorted(sources.items()):
        if relative_path in ratchet:
            continue
        actual_lines = line_count(path)
        if actual_lines > line_limit:
            errors.append(
                f"{relative_path}: {actual_lines} строк превышают порог "
                f"{line_limit}; разбейте файл или добавьте "
                "зафиксированный ratchet"
            )

    return errors


def main() -> int:
    errors = violations()
    if errors:
        for error in errors:
            print(f"source file size: {error}", file=sys.stderr)
        return 1

    print(
        f"source file size: ok (limit: {LINE_LIMIT}; ratchets: {len(RATCHET)})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
