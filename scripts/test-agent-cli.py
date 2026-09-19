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
qr_prompts = 0
try:
    while True:
        client, _ = server.accept()
        with client:
            data = client.makefile("rb").readline(16385)
            if not data: continue
            request = json.loads(data)
            kind = request["type"]
            stop_after_response = False
            with (db / "requests").open("a") as log: log.write(kind + "\n")
            if (db / "phone-login").exists() and kind.startswith("login_"):
                phase = (db / "phone-login").read_text()
                rotated = (db / "phone-rotated").exists()
                challenge = "auth-" + "1" * 32 + ("-0000000000000002" if rotated else "-0000000000000001")
                if kind == "login_submit":
                    assert request["challenge_id"] == challenge
                    if phase == "qr":
                        assert request["input"] == {"kind":"cancel_qr_code"}
                        if not rotated:
                            (db / "phone-rotated").touch()
                            client.sendall(json.dumps({"type":"command_error", "code":"login_challenge_invalid"}).encode() + b"\n")
                            continue
                        (db / "phone-login").write_text("phone")
                        stop_after_response = True
                    else:
                        assert phase == "phone" and request["input"] == {"kind":"phone_number", "value":"+15550000000"}
                        (db / "phone-login").write_text("ready")
                    response = {"type":"login_submitted", "challenge_id":challenge}
                elif kind == "login_prompt":
                    assert phase == "phone"
                    response = {"type":"login_prompt", "challenge_id":challenge, "prompt":{"kind":"phone_number"}}
                else:
                    response = {"type":"login_status", "state":{"qr":"qr_code","phone":"phone_number","ready":"ready"}[phase], "challenge_id":None if phase == "ready" else challenge, "next_action":"ready" if phase == "ready" else "submit_via_protected_channel"}
            elif (db / "qr-fixture").exists() and kind in ("login_status", "login_prompt"):
                challenge = "auth-" + "0" * 32 + "-" + format(qr_prompts + 1, "016x")
                if kind == "login_prompt":
                    assert request["challenge_id"] == challenge
                    response = {"type":"login_prompt", "challenge_id":challenge, "prompt":{"kind":"qr_code", "link":"tg://login?token=" + "A" * 42 + str(qr_prompts)}}
                    qr_prompts += 1
                elif qr_prompts < 2:
                    response = {"type":"login_status", "state":"qr_code", "challenge_id":challenge, "next_action":"confirm_other_device"}
                else: response = {"type":"login_status", "state":"ready", "challenge_id":None, "next_action":"ready"}
            elif kind == "lease_acquire":
                assert request["scopes"] == ["read"] and request["ttl_ms"] <= 60000
                response = {"type":"lease_granted", "lease":{"lease_id":"test-lease", "principal":request["principal"], "scopes":["read"], "ttl_ms":60000, "expires_in_ms":60000}}
            elif kind == "lease_release": response = {"type":"lease_released", "lease_id":"test-lease"}
            elif kind == "workflow_run":
                if request["workflow"] == "uncertain": continue  # Simulate response loss AFTER dispatch.
                response = {"type":"workflow_result", "workflow":request["workflow"], "result":{"fixture":True}, "complete":True}
            else: response = {"type":"login_status", "state":"ready", "challenge_id":None, "next_action":"ready"}
            try: client.sendall(json.dumps(response).encode() + b"\n")
            except BrokenPipeError: pass
            if stop_after_response: break
finally:
    server.close(); path.unlink(missing_ok=True)
'''


def run(binary, arguments, environment, code=0):
    result = subprocess.run([str(binary), *arguments], env=environment, capture_output=True, text=True, timeout=15, start_new_session=True)
    assert result.returncode == code, (arguments, result.returncode, result.stdout, result.stderr)
    assert "canary-api" not in result.stdout + result.stderr
    return result


def run_owner(binary, arguments, environment, streams=None, answers=()):
    pid, master = pty.fork()
    if pid == 0:
        if streams is not None:
            for fd, path in zip((1, 2), streams):
                with path.open("wb") as stream: os.dup2(stream.fileno(), fd)
        os.execve(str(binary), [str(binary), *arguments], environment)
    deadline = time.monotonic() + 15
    output = bytearray()
    pending = iter(answers)
    answer = next(pending, None)
    # Deliberately let the small macOS TTY output queue fill before reading a QR.
    if streams is not None: time.sleep(0.2)
    try:
        while time.monotonic() < deadline:
            if select.select([master], [], [], 0.1)[0]:
                try: chunk = os.read(master, 65536)
                except OSError: break
                if not chunk: break
                output.extend(chunk)
                if answer is not None and answer[0].encode() in output:
                    os.write(master, answer[1].encode())
                    answer = next(pending, None)
            ended, status = os.waitpid(pid, os.WNOHANG)
            if ended:
                assert os.waitstatus_to_exitcode(status) == 0, output.decode(errors="replace")
                return output.decode()
        ended, status = os.waitpid(pid, os.WNOHANG)
        if not ended:
            os.kill(pid, signal.SIGKILL); os.waitpid(pid, 0)
            raise AssertionError("owner fixture timed out")
        assert os.waitstatus_to_exitcode(status) == 0, output.decode(errors="replace")
        return output.decode()
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
            # QR login refreshes in the owner's TTY without exposing tokens to agents or pipes.
            (db / "qr-fixture").touch()
            result = run(cli, ["--agent", "login"], environment)
            assert json.loads(result.stdout)["data"]["state"] == "qr_code"
            streams = [root / "stdout", root / "stderr"]
            terminal = run_owner(cli, ["login"], environment, streams)
            assert terminal.count("Отсканируйте") == 2
            assert terminal.count("\x1b[30;47m") == 2 and "█" in terminal
            assert "tg://login" not in terminal
            for output in (result.stdout + result.stderr, *(path.read_text() for path in streams)):
                assert "tg://login" not in output and "█" not in output and "Отсканируйте" not in output
            (db / "phone-login").write_text("qr")
            run(cli, ["--agent", "login", "phone"], environment, 2)
            assert (db / "phone-login").read_text() == "qr"
            terminal = run_owner(cli, ["login", "phone"], environment, streams,
                [("Телефон в международном формате:", "+15550000000\n")])
            assert "Войти по QR" not in terminal and "█" not in terminal
            assert (db / "phone-login").read_text() == "ready"
            assert saved.read_bytes() == original
            assert all("+15550000000" not in path.read_text() for path in streams)
        finally:
            if (db / "pid").exists():
                try: os.kill(int((db / "pid").read_text()), signal.SIGTERM)
                except ProcessLookupError: pass
            Path(f"/tmp/telegramd-{os.geteuid()}/{profile}.sock").unlink(missing_ok=True)
    print("agent CLI: discovery, setup, reuse/restart, leases, skills and private QR refresh ok")


if __name__ == "__main__":
    main()
