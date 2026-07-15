#!/usr/bin/env python3
"""Negative controls для exact commit identity при сборке source archive."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import stat
import sys
import tempfile


ROOT = Path(__file__).resolve().parent.parent
BUILDER_PATH = ROOT / "scripts/build-tdlib-native.py"
COMMIT = "07d3a0973f5113b0827a04d54a93aaaa9e288348"


def load_builder():
    spec = importlib.util.spec_from_file_location("tdlib_native_builder", BUILDER_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("native builder module не загружен")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main() -> int:
    builder = load_builder()
    required = (
        "inject_exact_git_head",
        "verify_generated_commit_hash",
    )
    missing = [name for name in required if not hasattr(builder, name)]
    if missing:
        print(
            "tdlib native commit test: отсутствуют " + ", ".join(missing),
            file=sys.stderr,
        )
        return 1

    with tempfile.TemporaryDirectory(prefix="tdlib-native-commit-") as directory:
        root = Path(directory)
        source = root / "source"
        source.mkdir()
        injection = builder.inject_exact_git_head(source, COMMIT)
        head = source / ".git/HEAD"
        if head.read_text(encoding="ascii") != COMMIT + "\n":
            print("tdlib native commit test: HEAD отличается", file=sys.stderr)
            return 1
        if stat.S_IMODE(head.lstat().st_mode) != 0o444:
            print("tdlib native commit test: HEAD mode не 0444", file=sys.stderr)
            return 1
        if (
            injection["commit"] != COMMIT
            or injection["strategy"] != "synthetic-detached-head"
        ):
            print("tdlib native commit test: injection record отличается", file=sys.stderr)
            return 1

        template = source / "GitCommitHash.cpp.in"
        generated = root / "GitCommitHash.cpp"
        template.write_text('return "@TD_GIT_COMMIT_HASH@";\n', encoding="utf-8")
        generated.write_text(f'return "{COMMIT}";\n', encoding="utf-8")
        generated_record = builder.verify_generated_commit_hash(
            template, generated, COMMIT
        )
        if any(len(value) != 64 for value in generated_record.values()):
            print("tdlib native commit test: generated hash invalid", file=sys.stderr)
            return 1

        generated.write_text('return "GITDIR-NOTFOUND";\n', encoding="utf-8")
        try:
            builder.verify_generated_commit_hash(template, generated, COMMIT)
        except builder.NativeBuildError:
            pass
        else:
            print("tdlib native commit test: wrong commit принят", file=sys.stderr)
            return 1

        second_source = root / "preexisting"
        (second_source / ".git").mkdir(parents=True)
        try:
            builder.inject_exact_git_head(second_source, COMMIT)
        except builder.NativeBuildError:
            pass
        else:
            print("tdlib native commit test: preexisting .git принят", file=sys.stderr)
            return 1

    print("tdlib native commit test: ok (injection=1, negative_controls=2)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
