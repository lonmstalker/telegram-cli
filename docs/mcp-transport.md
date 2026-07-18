# MCP transport contract

`telegram-mcp` использует официальный Rust SDK `rmcp 2.2.0` и закреплённый MCP
protocol `2025-11-25`. Один runtime обслуживает standard newline-delimited stdio;
remote mode запускает тот же stdio через OpenSSH и не открывает TCP/HTTP listener.

## Local stdio

```sh
telegram-mcp stdio
```

- Principal выводится как `local:<effective_uid>`, а profile берётся из
  `TELEGRAM_PROFILE` (`default` при отсутствии).
- `TELEGRAM_MCP_SCOPES` задаёт transport ceiling; отсутствие или пустое значение
  означает только `read`. Daemon owner ceiling и generated policy остаются
  следующими независимыми gates.
- Startup, initialize и `tools/list` не подключаются к daemon и не создают TDLib client.
  Только `tools/call` делает один request к existing private profile socket.

## Remote over OpenSSH

Remote key обязан быть restricted forced command, например:

```text
restrict,command="/usr/local/bin/telegram-mcp ssh-stdio reader" ssh-ed25519 <public-key>
```

MCP client запускает `ssh <host> telegram-mcp`; OpenSSH аутентифицирует key, шифрует
весь stdio channel и устанавливает `SSH_CONNECTION`. Binary отказывает remote mode,
если sshd context отсутствует. Process работает под тем же OS user, что и `telegramd`,
поэтому использует existing owner-only `0600` socket, но не TDLib DB.

Forced identity `reader` выбирает только fixed policy
`/etc/telegram-cli/mcp-ssh/reader.json`:

```json
{"profile":"default","scopes":["read"]}
```

Directory обязан быть root-owned exact `0755`, policy — root-owned regular file exact
`0644` с одним hard link. Symlink, небезопасный mode, unknown field, invalid profile,
empty/unknown scopes fail closed. Principal становится `ssh:reader`; tool arguments не
могут подменить principal, profile или transport ceiling. Remote key не должен получать
shell, PTY, forwarding или возможность выбирать другой policy ID.

## Protected operator login

`auth.begin/status/wait` возвращают только opaque string `challenge_id`, `LoginState` и typed
`next_action`. Когда action равен `submit_via_protected_channel`, владелец открывает
отдельную, независимо аутентифицированную SSH-сессию с PTY и запускает на сервере:

```sh
telegram-cli login tty <challenge_id>
```

Это не MCP forced-command key и не model terminal. Phone/OTP/email/registration видны owner
в server `/dev/tty`; echo cloud password выбирает сам owner. Input идёт только внутри зашифрованного SSH
channel; в arguments находится
лишь non-secret boot-scoped token. CLI сверяет token до prompt и отправляет ровно один typed
input. После restart/profile switch старый token fail closed. После этого MCP использует
`auth.wait` для следующего token/action или доказанного `Ready`.

## Reconnect and results

OpenSSH reconnect создаёт новую MCP connection и новый lifecycle. Adapter не хранит
бизнес-состояние и не replay-ит request. Daemon machine envelope остаётся источником
`partial`, `gap` и `reconciliation_required`; uncertain mutation нельзя повторять только
из-за разрыва SSH/MCP transport.

## Verification

- Unit tests проверяют protocol translation, forged principal denial, transport scope
  ceiling и root-policy mode/symlink boundary.
- Synthetic stdio trace выполняет `initialize -> notifications/initialized -> tools/list`
  без daemon и возвращает восемь tools.
- `ssh-stdio` без `SSH_CONNECTION` возвращает non-zero, ничего не пишет в stdout и не
  пытается открыть daemon socket.
