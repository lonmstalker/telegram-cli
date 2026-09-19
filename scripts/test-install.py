#!/usr/bin/env python3
"""Verify release installation in temporary HOME/prefix; needs the pinned native cache."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent


def main():
    with tempfile.TemporaryDirectory(prefix="telegram-install-check-") as temporary:
        stage = Path(temporary)
        (stage / "home").mkdir()
        environment = {"HOME": str(stage / "home"), "PATH": os.environ.get("PATH", "")}

        def check(arguments):
            result = subprocess.run(arguments, cwd=ROOT, env=environment,
                                    capture_output=True, text=True, timeout=30)
            if result.returncode:
                raise SystemExit(result.stdout + result.stderr)
            return result.stdout

        check(["sh", "install.sh", "--no-build", "--prefix", str(stage / "prefix"), "--skill", "both"])
        cli = stage / "prefix/bin/telegram-cli"
        assert json.loads(check([str(cli), "--agent", "doctor"]))["data"]["configured"] is False
        assert json.loads(check([str(cli), "--agent", "workflow", "describe", "user_profile"]))["status"] == "ok"
        for kind in (".agents", ".claude"):
            assert (stage / "home" / kind / "skills/telegram-cli/SKILL.md").is_file()
        (stage / "aliases").mkdir()
        alias = stage / "aliases/tg"
        alias.symlink_to(cli)
        assert json.loads(check([str(alias), "--agent", "workflow", "list"]))["status"] == "ok"
        bad_native = stage / "bad-native"; bad_native.write_bytes(b"not pinned")
        rejected = subprocess.run(["sh", "install.sh", "--no-build", "--prefix", str(stage / "rejected"), "--native", str(bad_native)], cwd=ROOT, env=environment, capture_output=True)
        assert rejected.returncode != 0 and not (stage / "rejected").exists()
        archive = stage / "bundle.tar.gz"
        check(["sh", "scripts/package-release.sh", str(archive)])
        check(["tar", "-xzf", str(archive), "-C", str(stage)])
        check(["sh", str(stage / "telegram-cli/install.sh"), "--prefix", str(stage / "bundle-install")])
        cli = stage / "bundle-install/bin/telegram-cli"
        assert json.loads(check([str(cli), "--agent", "schema", "version"]))["data"]["version"]["runtime_verified"] is False
    print("install: source layout, both skills, bundle reinstall and offline discovery passed")


if __name__ == "__main__":
    main()
