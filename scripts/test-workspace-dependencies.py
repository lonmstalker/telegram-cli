#!/usr/bin/env python3
"""Positive и negative controls для workspace-dependencies guard."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import sys


ROOT = Path(__file__).resolve().parent.parent


def load_checker():
    path = ROOT / "scripts/check-workspace-dependencies.py"
    spec = importlib.util.spec_from_file_location("workspace_dependencies_checker", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"module не загружен: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def write(path: Path, source: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(source, encoding="utf-8")


def main() -> int:
    checker = load_checker()
    with tempfile.TemporaryDirectory(prefix="workspace-dependencies-") as directory:
        fixture = Path(directory)
        write(
            fixture / "Cargo.toml",
            """[workspace]
members = ["crates/*", "apps/*"]

[workspace.dependencies]
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.150"
""",
        )
        write(
            fixture / "crates/good/Cargo.toml",
            """[package]
name = "good"
version = "0.1.0"

[dependencies]
serde = { workspace = true, features = ["rc"] }
base64 = "0.22"

[target.'cfg(unix)'.dev-dependencies]
serde_json.workspace = true
""",
        )
        write(
            fixture / "crates/good-subtable/Cargo.toml",
            """[package]
name = "good-subtable"
version = "0.1.0"

[dependencies.serde_json]
workspace = true
features = ["preserve_order"]
""",
        )
        if errors := checker.violations(fixture):
            print(
                "workspace dependencies test: positive fixture rejected: "
                f"{errors}",
                file=sys.stderr,
            )
            return 1

        write(
            fixture / "apps/bad/Cargo.toml",
            """[package]
name = "bad"
version = "0.1.0"

[dependencies]
serde = "1.0.228"

[build-dependencies]
serde_json = { version = "1.0.150" }
""",
        )
        write(
            fixture / "apps/bad-subtable/Cargo.toml",
            """[package]
name = "bad-subtable"
version = "0.1.0"

[dependencies.serde]
version = "1.0.228"
""",
        )
        errors = checker.violations(fixture)
        if len(errors) != 3 or not all("explicit version" in error for error in errors):
            print(
                f"workspace dependencies test: explicit versions accepted: {errors}",
                file=sys.stderr,
            )
            return 1

    print("workspace dependencies test: ok (positive=2, negative=3)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
