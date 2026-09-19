#!/usr/bin/env python3
"""Cold install/profile/agent checks with a fake daemon; never contacts Telegram."""
import json
import os
from pathlib import Path
import shutil
import pty
import select
from concurrent.futures import ThreadPoolExecutor
import signal
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent
BIN = ROOT / "target/debug/telegram-cli"

FAKE_DAEMON = r'''
import json, os, pathlib, socket, sys, time, fcntl
if sys.argv[1:] == ["--check-native", sys.argv[-1]]:
    sys.exit(0)
db = pathlib.Path(os.environ["TDLIB_DATABASE_DIR"])
lock = (db / "owner.lock").open("w")
try: fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
except BlockingIOError: sys.exit(1)
if (db / "delay").exists(): time.sleep(float((db / "delay").read_text()))
(db / "pid").write_text(str(os.getpid()))
with (db / "starts").open("a") as log: log.write("start\n")
profile = os.environ["TELEGRAM_PROFILE"]
directory = pathlib.Path(f"/tmp/telegramd-{os.geteuid()}")
directory.mkdir(mode=0o700, exist_ok=True)
path = directory / (profile + ".sock")
path.unlink(missing_ok=True)
server = socket.socket(socket.AF_UNIX)
server.bind(str(path)); path.chmod(0o600); server.listen(); server.settimeout(10)
try:
    while True:
        client, _ = server.accept()
        with client:
            data = client.makefile("rb").readline(16385)
            if not data: continue
            request = json.loads(data)
            kind = request["type"]
            with (db / "requests").open("a") as log: log.write(kind + "\n")
            if kind == "lease_acquire":
                assert request["scopes"] == ["read"] and request["ttl_ms"] <= 60000
                response = {"type":"lease_granted", "lease":{"lease_id":"test-lease", "principal":request["principal"], "scopes":["read"], "ttl_ms":60000, "expires_in_ms":60000}}
            elif kind == "lease_release": response = {"type":"lease_released", "lease_id":"test-lease"}
            elif kind == "workflow_run":
                if request["workflow"] == "uncertain": continue  # Simulate response loss AFTER dispatch.
                response = {"type":"workflow_result", "workflow":request["workflow"], "result":{"fixture":True}, "complete":True}
            else: response = {"type":"login_status", "state":"ready", "challenge_id":None, "next_action":"ready"}
            try: client.sendall(json.dumps(response).encode() + b"\n")
            except BrokenPipeError: pass
finally:
    server.close(); path.unlink(missing_ok=True)
'''


def run(binary, arguments, environment, code=0):
    result = subprocess.run([str(binary), *arguments], env=environment, capture_output=True, text=True, timeout=15, start_new_session=True)
    assert result.returncode == code, (arguments, result.returncode, result.stdout, result.stderr)
    assert "canary-api" not in result.stdout + result.stderr
    return result


def run_owner(binary, arguments, environment):
    pid, master = pty.fork()
    if pid == 0:
        os.execve(str(binary), [str(binary), *arguments], environment)
    deadline = time.monotonic() + 15
    output = bytearray()
    try:
        while time.monotonic() < deadline:
            if select.select([master], [], [], 0.1)[0]:
                try: chunk = os.read(master, 4096)
                except OSError: break
                if not chunk: break
                output.extend(chunk)
            ended, status = os.waitpid(pid, os.WNOHANG)
            if ended:
                assert os.waitstatus_to_exitcode(status) == 0, output.decode(errors="replace")
                return
        ended, status = os.waitpid(pid, os.WNOHANG)
        if not ended:
            os.kill(pid, signal.SIGKILL); os.waitpid(pid, 0)
            raise AssertionError("owner fixture timed out")
        assert os.waitstatus_to_exitcode(status) == 0, output.decode(errors="replace")
    finally:
        os.close(master)


def stop_daemon(db):
    pid = int((db / "pid").read_text())
    os.kill(pid, signal.SIGKILL)
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        try: os.kill(pid, 0)
        except ProcessLookupError: return
        time.sleep(0.01)
    raise AssertionError("fixture daemon did not exit")


def main():
    assert BIN.is_file(), "Build telegram-cli and telegramd first"
    with tempfile.TemporaryDirectory(prefix="telegram-agent-test-") as temporary:
        root = Path(temporary)
        environment = {"HOME": temporary, "PATH": os.environ.get("PATH", "")}
        for args in (["--help"], ["--version"], ["--agent", "schema", "version"], ["--agent", "workflow", "list"], ["--agent", "workflow", "describe", "user_profile"]):
            run(BIN, args, environment)
        bad_profile = dict(environment, TELEGRAM_PROFILE=b"\xff")
        assert json.loads(run(BIN, ["--agent", "doctor"], bad_profile, 2).stdout)["error"]["code"] == "invalid_profile"
        result = run(BIN, ["--agent", "doctor"], environment)
        assert json.loads(result.stdout)["data"]["configured"] is False
        result = run(BIN, ["--agent", "setup"], environment, 2)
        assert json.loads(result.stdout)["error"]["code"] == "invalid_arguments"
        for args in (["init", "--global"], ["init", "--global", "--claude"]):
            run(BIN, args, environment)
        for directory in (".agents", ".claude"):
            assert (root / directory / "skills/telegram-cli/SKILL.md").read_text() == (ROOT / ".agents/skills/telegram-cli/SKILL.md").read_text()

        (root / "bin").mkdir()
        cli = root / "bin/telegram-cli"
        shutil.copy2(BIN, cli)
        daemon = root / "bin/telegramd"
        daemon.write_text("#!" + sys.executable + "\n" + FAKE_DAEMON)
        daemon.chmod(0o755)
        db = root / "database"; db.mkdir(mode=0o700)
        native = root / "native"; native.touch()
        key = root / "key"; key.write_text("dGVzdA=="); key.chmod(0o600)
        profile = "agent-test-" + str(os.getpid())
        fixture = dict(environment, TELEGRAM_PROFILE=profile, TELEGRAM_API_ID="1", TELEGRAM_API_HASH="a" * 32,
            TDLIB_DATABASE_DIR=str(db), TDLIB_FILES_DIR=str(db), TDLIB_DATABASE_KEY_FILE=str(key), TDJSON_LIBRARY_PATH=str(native))
        environment["TELEGRAM_PROFILE"] = profile
        try:
            result = run(cli, ["--agent", "login"], fixture, 3)
            assert json.loads(result.stdout)["error"]["code"] == "profile_not_configured"
            run(cli, ["setup", "--import-env"], fixture, 3)
            run_owner(cli, ["setup", "--import-env"], fixture)
            saved = root / ".config/telegram-cli" / profile / "profile.json"
            original = saved.read_bytes()
            assert saved.stat().st_mode & 0o777 == 0o600
            # No API credentials in the subsequent agent's environment.
            assert json.loads(run(cli, ["--agent", "login"], environment).stdout)["data"]["state"] == "ready"
            result = run(cli, ["--agent", "run", "user_profile", '{"target":{"kind":"self"}}'], environment)
            assert json.loads(result.stdout)["data"]["complete"] is True
            # Response loss still releases the lease and does not replay the operation.
            run(cli, ["--agent", "run", "uncertain", "{}"], environment, 5)
            requests = (db / "requests").read_text().splitlines()
            assert requests.count("workflow_run") == 2
            assert requests[-1] == "lease_release"
            assert (db / "starts").read_text().splitlines() == ["start"]
            # Restart reuses the saved profile and key; setup rerun doesn't replace either.
            stop_daemon(db)
            assert Path(f"/tmp/telegramd-{os.geteuid()}/{profile}.sock").exists(), "expected stale socket"
            (db / "delay").write_text("2.5")
            with ThreadPoolExecutor(max_workers=2) as pool:
                results = list(pool.map(lambda _: run(cli, ["--agent", "login"], environment), range(2)))
            assert all(json.loads(result.stdout)["data"]["state"] == "ready" for result in results)
            run(cli, ["setup", "--import-env"], fixture)
            assert saved.read_bytes() == original
            assert len((db / "starts").read_text().splitlines()) == 2
        finally:
            if (db / "pid").exists():
                try: os.kill(int((db / "pid").read_text()), signal.SIGTERM)
                except ProcessLookupError: pass
            Path(f"/tmp/telegramd-{os.geteuid()}/{profile}.sock").unlink(missing_ok=True)
    print("agent CLI: offline discovery, protected setup, reuse/restart, lease cleanup and skills ok")


if __name__ == "__main__":
    main()
