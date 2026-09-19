#!/usr/bin/env python3
"""Verify release installation in temporary HOME/prefix; needs the pinned native cache."""
import json
import hashlib
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parent.parent


def check_download_install(stage, archive):
    """Use the real bundle; fake only HTTPS downloads, with no build tools on PATH."""
    tools = stage / "runtime-tools"; tools.mkdir()
    for name in ("sh", "uname", "tar", "mktemp", "rm", "dirname", "mkdir", "install", "mv", "shasum", "sha256sum"):
        if executable := shutil.which(name):
            (tools / name).symlink_to(executable)
    curl = tools / "curl"
    curl.write_text(f"#!{sys.executable}\n" + '''import os, pathlib, shutil, sys
url = next(arg for arg in sys.argv[1:] if arg.startswith("https://"))
assert url.startswith("https://github.com/lonmstalker/telegram-cli/releases/")
pathlib.Path(os.environ["DOWNLOAD_LOG"]).open("a").write(url + "\\n")
if os.environ.get("DOWNLOAD_FAIL"): sys.exit(22)
shutil.copyfile(pathlib.Path(os.environ["DOWNLOAD_FIXTURE"]) / url.rsplit("/", 1)[1], sys.argv[sys.argv.index("-o") + 1])
''')
    curl.chmod(0o755)
    target = "aarch64-apple-darwin" if platform.system() == "Darwin" else "x86_64-unknown-linux-gnu"
    assets = stage / "assets"; assets.mkdir()
    asset = assets / f"telegram-cli-{target}.tar.gz"
    shutil.copyfile(archive, asset)
    checksum = asset.with_name(asset.name + ".sha256")
    checksum.write_text(f"{hashlib.sha256(asset.read_bytes()).hexdigest()}  {asset.name}\n")
    home = stage / "download-home"; home.mkdir()
    environment = {"HOME": str(home), "PATH": str(tools), "DOWNLOAD_FIXTURE": str(assets),
                   "DOWNLOAD_LOG": str(stage / "download-urls")}
    assert shutil.which("cargo", path=str(tools)) is None
    assert shutil.which("python3", path=str(tools)) is None
    source = (ROOT / "scripts/install-release.sh").read_text()

    def invoke(*args, fail=False):
        result = subprocess.run([str(tools / "sh"), "-s", "--", *args], input=source,
                                env=environment, capture_output=True, text=True, timeout=30)
        assert (result.returncode != 0) == fail, result.stdout + result.stderr
        return result

    invoke()  # Default prefix + both skills, as with curl | sh.
    invoke()  # Reinstall over existing binaries and skills must not prompt or fail.
    cli = home / ".local/bin/telegram-cli"
    assert cli.is_file() and (cli.parent / "telegramd").is_file()
    for kind in (".agents", ".claude"):
        assert (home / kind / "skills/telegram-cli/SKILL.md").is_file()
    result = subprocess.run([str(cli), "--agent", "workflow", "list"], env=environment,
                            capture_output=True, text=True, timeout=30)
    assert result.returncode == 0 and json.loads(result.stdout)["status"] == "ok"
    invoke("--version", "v0.1.0", "--prefix", str(stage / "custom prefix"), "--skill", "none")
    urls = Path(environment["DOWNLOAD_LOG"]).read_text()
    assert "/latest/download/" in urls and "/download/v0.1.0/" in urls
    rejected = stage / "must-not-install"
    checksum.write_text("")
    assert "Empty checksum file" in invoke("--prefix", str(rejected), fail=True).stderr
    checksum.write_text(f"{'0' * 64}  {asset.name}\n")
    assert "SHA-256 mismatch" in invoke("--prefix", str(rejected), fail=True).stderr
    assert not rejected.exists()
    environment["DOWNLOAD_FAIL"] = "1"
    assert "Cannot download" in invoke("--prefix", str(rejected), fail=True).stderr
    assert not rejected.exists()
    del environment["DOWNLOAD_FAIL"]
    with tarfile.open(asset, "w:gz") as bundle:
        bundle.addfile(tarfile.TarInfo("../escaped"))
    checksum.write_text(f"{hashlib.sha256(asset.read_bytes()).hexdigest()}  {asset.name}\n")
    assert "Unsafe bundle path" in invoke("--prefix", str(rejected), fail=True).stderr
    assert not rejected.exists()
    before = Path(environment["DOWNLOAD_LOG"]).read_text()
    invoke("--version", "../main", fail=True)
    invoke("--prefix", "relative", fail=True)
    invoke("--skill", "unknown", fail=True)
    assert Path(environment["DOWNLOAD_LOG"]).read_text() == before
    truncated = subprocess.run([str(tools / "sh"), "-s"], input=source[:len(source) // 2],
                               env=environment, capture_output=True, text=True, timeout=30)
    assert truncated.returncode != 0 and Path(environment["DOWNLOAD_LOG"]).read_text() == before


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
        assert archive.with_name(archive.name + ".sha256").read_text() == f"{hashlib.sha256(archive.read_bytes()).hexdigest()}  {archive.name}\n"
        with tarfile.open(archive) as bundle:
            assert all(item.uid == item.gid == 0 and item.uname in ("", "root") and item.gname in ("", "root") for item in bundle)
        check(["tar", "-xzf", str(archive), "-C", str(stage)])
        check(["sh", str(stage / "telegram-cli/install.sh"), "--prefix", str(stage / "bundle-install")])
        cli = stage / "bundle-install/bin/telegram-cli"
        assert json.loads(check([str(cli), "--agent", "schema", "version"]))["data"]["version"]["runtime_verified"] is False
        check_download_install(stage, archive)
    print("install: source/bundle/download, both skills, no build tools, checksum and failure checks passed")


if __name__ == "__main__":
    main()
