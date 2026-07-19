#!/usr/bin/env python3
"""Требует наследовать общие Cargo dependencies через workspace."""

from __future__ import annotations

from pathlib import Path
import re
import sys
from typing import Optional


ROOT = Path(__file__).resolve().parent.parent
DEPENDENCY_TABLES = ("dependencies", "dev-dependencies", "build-dependencies")
SOURCE_FIELDS = ("version", "path", "git", "registry")
QUOTED_STRING = re.compile(r"[\"']([^\"']+)[\"']")


def strip_comment(line: str) -> str:
    quote = None
    escaped = False
    for index, character in enumerate(line):
        if escaped:
            escaped = False
            continue
        if character == "\\" and quote == '"':
            escaped = True
            continue
        if quote:
            if character == quote:
                quote = None
            continue
        if character in ('"', "'"):
            quote = character
        elif character == "#":
            return line[:index]
    return line


def balance(value: str) -> int:
    result = 0
    quote = None
    escaped = False
    for character in value:
        if escaped:
            escaped = False
            continue
        if character == "\\" and quote == '"':
            escaped = True
            continue
        if quote:
            if character == quote:
                quote = None
            continue
        if character in ('"', "'"):
            quote = character
        elif character in "[{":
            result += 1
        elif character in "]}":
            result -= 1
    return result


def statements(path: Path) -> list[tuple[str, str, str, int]]:
    """Возвращает (section, key, value, line) без полного TOML parser."""
    result: list[tuple[str, str, str, int]] = []
    section = ""
    pending = ""
    pending_line = 0
    pending_balance = 0

    for line_number, physical_line in enumerate(
        path.read_text(encoding="utf-8").splitlines(), start=1
    ):
        line = strip_comment(physical_line).strip()
        if not line:
            continue
        if not pending and line.startswith("[") and line.endswith("]"):
            section = line[1:-1].strip()
            continue

        if not pending:
            pending = line
            pending_line = line_number
            pending_balance = balance(line)
        else:
            pending = f"{pending} {line}"
            pending_balance += balance(line)
        if pending_balance > 0:
            continue

        key, separator, value = pending.partition("=")
        if separator:
            result.append((section, key.strip(), value.strip(), pending_line))
        pending = ""
        pending_balance = 0

    if pending:
        raise ValueError(
            f"{path}:{pending_line}: незавершённое TOML-выражение"
        )
    return result


def unquote(value: str) -> str:
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in ('"', "'"):
        return value[1:-1]
    return value


def split_key(key: str) -> list[str]:
    parts: list[str] = []
    current = ""
    quote = None
    for character in key:
        if quote:
            current += character
            if character == quote:
                quote = None
            continue
        if character in ('"', "'"):
            quote = character
            current += character
        elif character == ".":
            parts.append(unquote(current))
            current = ""
        else:
            current += character
    parts.append(unquote(current))
    return [part.strip() for part in parts]


def inline_field(value: str, field: str) -> Optional[str]:
    match = re.search(
        rf"(?:^|[,{{])\s*{re.escape(field)}\s*=\s*([^,}}]+)", value
    )
    return match.group(1).strip() if match else None


def dependency_table(section: str) -> bool:
    return split_key(section)[-1] in DEPENDENCY_TABLES


def workspace_contract(
    root: Path,
) -> tuple[list[str], set[str], set[str], set[str]]:
    root_statements = statements(root / "Cargo.toml")
    members: list[str] = []
    excluded: set[str] = set()
    shared: dict[str, str] = {}

    for section, key, value, _ in root_statements:
        if section == "workspace" and key in ("members", "exclude"):
            values = QUOTED_STRING.findall(value)
            if key == "members":
                members.extend(values)
            else:
                excluded.update(values)
        elif section == "workspace.dependencies":
            key_parts = split_key(key)
            dependency = key_parts[0]
            shared.setdefault(dependency, dependency)
            if len(key_parts) == 1:
                package = inline_field(value, "package")
                if package:
                    shared[dependency] = unquote(package)
            elif key_parts[1] == "package":
                shared[dependency] = unquote(value)
        else:
            section_parts = split_key(section)
            if (
                section_parts[:2] == ["workspace", "dependencies"]
                and len(section_parts) == 3
            ):
                dependency = section_parts[2]
                shared.setdefault(dependency, dependency)
                if key == "package":
                    shared[dependency] = unquote(value)

    return members, excluded, set(shared), set(shared.values())


def member_manifests(root: Path, members: list[str], excluded: set[str]) -> list[Path]:
    manifests: set[Path] = set()
    excluded_paths = {
        candidate.resolve()
        for pattern in excluded
        for candidate in root.glob(pattern)
    }
    for pattern in members:
        for candidate in root.glob(pattern):
            if candidate.resolve() in excluded_paths:
                continue
            manifest = (
                candidate
                if candidate.name == "Cargo.toml"
                else candidate / "Cargo.toml"
            )
            if manifest.is_file():
                manifests.add(manifest)
    return sorted(manifests)


def violations(root: Path = ROOT) -> list[str]:
    members, excluded, shared_keys, shared_packages = workspace_contract(root)
    if not members:
        return ["root Cargo.toml: [workspace].members пуст или отсутствует"]
    if not shared_keys:
        return ["root Cargo.toml: [workspace.dependencies] пуст или отсутствует"]

    errors: list[str] = []
    for manifest_path in member_manifests(root, members, excluded):
        declarations: dict[tuple[str, str], dict[str, object]] = {}
        for section, key, value, line_number in statements(manifest_path):
            section_parts = split_key(section)
            if dependency_table(section):
                table_name = section
                section_dependency = None
            elif (
                len(section_parts) >= 2
                and section_parts[-2] in DEPENDENCY_TABLES
            ):
                table_name = ".".join(section_parts[:-1])
                section_dependency = section_parts[-1]
            else:
                continue
            key_parts = split_key(key)
            dependency = section_dependency or key_parts[0]
            declaration = declarations.setdefault(
                (table_name, dependency),
                {
                    "workspace": False,
                    "sources": set(),
                    "package": dependency,
                    "line": line_number,
                },
            )
            field = (
                key_parts[0]
                if section_dependency is not None
                else key_parts[1] if len(key_parts) > 1 else None
            )
            if field == "workspace":
                declaration["workspace"] = value == "true"
            elif field == "package":
                declaration["package"] = unquote(value)
            elif field in SOURCE_FIELDS:
                declaration["sources"].add(field)
            elif field is None:
                workspace_value = inline_field(value, "workspace")
                declaration["workspace"] = workspace_value == "true"
                package = inline_field(value, "package")
                if package:
                    declaration["package"] = unquote(package)
                inline_sources = {
                    source for source in SOURCE_FIELDS if inline_field(value, source)
                }
                if value.startswith(('"', "'")):
                    inline_sources.add("version")
                declaration["sources"].update(inline_sources)

        for (table_name, dependency), declaration in sorted(declarations.items()):
            package = declaration["package"]
            if dependency not in shared_keys and package not in shared_packages:
                continue
            sources = sorted(declaration["sources"])
            if declaration["workspace"] and not sources:
                continue
            detail = (
                f"explicit {', '.join(sources)}"
                if sources
                else "нет workspace = true"
            )
            relative_path = manifest_path.relative_to(root).as_posix()
            errors.append(
                f"{relative_path}:{declaration['line']} [{table_name}] {dependency}: "
                f"{detail}; используйте workspace = true"
            )
    return errors


def main() -> int:
    try:
        errors = violations()
    except (OSError, ValueError) as error:
        print(f"workspace dependencies: manifest read failed: {error}", file=sys.stderr)
        return 1

    if errors:
        for error in errors:
            print(f"workspace dependencies: {error}", file=sys.stderr)
        return 1

    print("workspace dependencies: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
