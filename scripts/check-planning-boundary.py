#!/usr/bin/env python3
"""Не допускает planning IDs в исполняемый код и machine-readable contracts."""

from __future__ import annotations

from pathlib import Path
import re
import sys
import tempfile


ROOT = Path(__file__).resolve().parent.parent
RUNTIME_ROOTS = (
    "apps",
    "crates",
    "tools",
    "policy",
    "generated",
    "scripts",
    ".cargo",
    ".github",
)
TEXT_SUFFIXES = {
    ".c",
    ".cc",
    ".cpp",
    ".graphql",
    ".h",
    ".hpp",
    ".js",
    ".json",
    ".lock",
    ".mjs",
    ".proto",
    ".py",
    ".ron",
    ".rs",
    ".sh",
    ".sql",
    ".toml",
    ".ts",
    ".tsx",
    ".yaml",
    ".yml",
}
TEXT_BASENAMES = {"Dockerfile", "Makefile"}
SELF = Path("scripts/check-planning-boundary.py")
NEGATIVE_CONTROL_COUNT = 7
MAX_FILE_BYTES = 4 * 1024 * 1024
FORBIDDEN = (
    (re.compile(r"\bFeatureId\b"), "planning type `FeatureId`"),
    (re.compile(r"\bfeature_id\b"), "planning field `feature_id`"),
    (re.compile(r"tdlib-feature-owners"), "feature-owner artifact"),
    (re.compile(r"\bF\d{3}\b"), "numeric planning ID"),
)


def violations(files: dict[str, str]) -> list[str]:
    found: list[str] = []
    for path, text in sorted(files.items()):
        inspected = f"{path}\n{text}"
        for pattern, label in FORBIDDEN:
            if pattern.search(inspected):
                found.append(f"{path}: {label}")
    return found


def repository_files(root: Path = ROOT) -> dict[str, str]:
    files: dict[str, str] = {}
    for root_name in RUNTIME_ROOTS:
        inspected_root = root / root_name
        if inspected_root.is_symlink():
            raise ValueError(f"{root_name}: symlink is forbidden in inspected paths")
        if not inspected_root.exists():
            continue
        for path in inspected_root.rglob("*"):
            relative = path.relative_to(root)
            if relative == SELF:
                continue
            if path.is_symlink():
                raise ValueError(f"{relative}: symlink is forbidden in inspected paths")
            if not path.is_file():
                continue
            if path.suffix not in TEXT_SUFFIXES and path.name not in TEXT_BASENAMES:
                continue
            size = path.stat().st_size
            if size > MAX_FILE_BYTES:
                raise ValueError(
                    f"{relative}: {size} bytes exceeds inspection cap"
                )
            files[str(relative)] = path.read_text(encoding="utf-8")
    for path in root.iterdir():
        relative = path.relative_to(root)
        if path.is_symlink():
            raise ValueError(f"{relative}: symlink is forbidden in inspected paths")
        if not path.is_file():
            continue
        if path.suffix not in TEXT_SUFFIXES and path.name not in TEXT_BASENAMES:
            continue
        size = path.stat().st_size
        if size > MAX_FILE_BYTES:
            raise ValueError(f"{relative}: {size} bytes exceeds inspection cap")
        files[str(relative)] = path.read_text(encoding="utf-8")
    return files


def negative_control_errors() -> list[str]:
    controls = {
        "FeatureId": {"crates/core/src/lib.rs": "pub enum FeatureId {}"},
        "feature_id": {"policy/capabilities.yaml": "feature_id: semantic"},
        "artifact": {"generated/tdlib-feature-owners.json": "{}"},
        "numeric_id": {"scripts/generate.sh": 'PLANNING_OWNER="F007"'},
        "root_numeric_id": {"build.rs": 'const PLANNING_OWNER: &str = "F007";'},
    }
    errors = []
    with tempfile.TemporaryDirectory(prefix="telegram-cli-planning-boundary-") as directory:
        temporary_root = Path(directory)
        expected_files: dict[str, str] = {}
        for files in controls.values():
            for relative, text in files.items():
                path = temporary_root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(text, encoding="utf-8")
                expected_files[relative] = text
        discovered = repository_files(temporary_root)
        for name, files in controls.items():
            relative = next(iter(files))
            if discovered.get(relative) != expected_files[relative]:
                errors.append(f"negative control `{name}` was not discovered")
            elif not violations({relative: discovered[relative]}):
                errors.append(f"negative control `{name}` was not detected")
        symlink_target = temporary_root / "symlink-target.txt"
        symlink_target.write_text('PLANNING_OWNER="F007"', encoding="utf-8")
        symlink_path = temporary_root / "scripts/linked.py"
        symlink_path.symlink_to(symlink_target)
        try:
            repository_files(temporary_root)
        except ValueError as error:
            if "symlink is forbidden" not in str(error):
                errors.append("negative control `symlink` raised an unrelated error")
        else:
            errors.append("negative control `symlink` was not rejected")
        symlink_root = temporary_root / "symlink-root"
        external_scripts = temporary_root / "external-scripts"
        symlink_root.mkdir()
        external_scripts.mkdir()
        (external_scripts / "generated.py").write_text(
            'PLANNING_OWNER="F007"', encoding="utf-8"
        )
        (symlink_root / "scripts").symlink_to(external_scripts, target_is_directory=True)
        try:
            repository_files(symlink_root)
        except ValueError as error:
            if "symlink is forbidden" not in str(error):
                errors.append("negative control `symlink_root` raised an unrelated error")
        else:
            errors.append("negative control `symlink_root` was not rejected")
    return errors


def main() -> int:
    try:
        errors = violations(repository_files())
    except (OSError, UnicodeError, ValueError) as error:
        print(f"planning boundary: {error}", file=sys.stderr)
        return 1
    errors.extend(negative_control_errors())
    if errors:
        for error in errors:
            print(f"planning boundary: {error}", file=sys.stderr)
        return 1
    print(f"planning boundary: ok (negative controls: {NEGATIVE_CONTROL_COUNT})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
