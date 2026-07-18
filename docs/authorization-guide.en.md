# Step-by-step Telegram CLI Authorization

[Русская версия](authorization-guide.ru.md) · [User guide](user-guide.en.md)

## Security boundary

Phone, OTP, email, and registration data are visible while being entered in the owner terminal. Before cloud password, the CLI asks `Скрыть cloud password? [y/N]:`: Enter/`n` keeps input visible and `y` disables echo. Do not send authorization values to an agent, chat, flags, stdin, environment, logs, or JSON. `telegram-cli login` reads values directly from `/dev/tty` and submits them to the daemon over the protected local channel.

`challenge_id` is not a secret. It is an opaque boot-scoped token for the current step; it changes
after every authorization update and after a restart/profile switch. It binds input to one exact
challenge and prevents stale submission.

## Before you begin

Build the applications, configure `.env.local`, and start the daemon as described in the [user guide](user-guide.en.md). One profile must be owned by only one `telegramd` process.

When using an existing TDLib database, first confirm that its correct encryption key is configured. A wrong key requires fixing the key reference, not starting a new phone authorization.

## Normal authorization with one command

The complete user flow runs through one command:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli login
```

The CLI reads the current state and proceeds through phone → OTP → 2FA/cloud password → email/registration when required by Telegram. You do not copy `challenge_id` or restart the command between steps. If the current code's server-specified timeout has elapsed and TDLib reports a next delivery method, the CLI requests a new code before prompting and waits for a new challenge. Input is visible by default; only cloud password has an owner-selected echo mode. The command returns after proven `ready`, an explicit error, or `Ctrl+C`.

The sections below document machine status, troubleshooting, and one-shot MCP/operator handoff. They are not needed for normal manual login.

## 1. Read the current state

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli --output json login
```

Inspect `data.state`, `data.challenge_id`, and `data.next_action`. A typical response may look like this:

```json
{
  "version": 4,
  "status": "partial",
  "data": {
    "type": "login_status",
    "state": "phone_number",
    "challenge_id": "auth-0123456789abcdef0123456789abcdef-0000000000000001",
    "next_action": "submit_via_protected_channel"
  }
}
```

The token string is only an example. Always use the value from a fresh daemon response.

For every login state other than `ready`, the root `status` is deliberately `partial`: authorization is not complete yet.

## 2. Perform the `next_action`

| `next_action` | What to do |
| --- | --- |
| `wait` | Enter nothing; wait briefly and request status again. |
| `submit_via_protected_channel` | Handle one challenge through `login tty <challenge_id>`; registration asks for first and last names. |
| `confirm_other_device` | Confirm the login in an already authorized Telegram client, then check status again. |
| `ready` | Authorization is proven; no more secret input is required. |
| `restart_daemon` | Gracefully restart the daemon and request status again. |

The daemon handles the `parameters` state from protected configuration; do not enter API credentials through TTY. The `logging_out`, `closing`, and temporary transition states normally require `wait`. The `closed` state requires restarting the daemon.

## 3. Submit one challenge through TTY

Replace the example with the current `challenge_id`:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli login tty auth-0123456789abcdef0123456789abcdef-0000000000000001
```

The command checks the ID before showing a prompt, handles one challenge, and returns control. Phone, OTP, email, and registration are visible in the owner terminal and remain in its scrollback. For cloud password, the owner chooses visible or hidden echo immediately before entry. A successful submission means only `login_submitted`, not completed authorization. Run the command from step 1 again and handle the new state.

If the ID is already stale, the command fails closed. Do not retry the old value: request a fresh status and use the new `challenge_id`.

## 4. Continue until `ready`

Telegram determines the exact sequence. It may include:

1. `phone_number` or `premium_purchase` — the account phone number for the current phone challenge;
2. `code` — an OTP delivered through Telegram/SMS;
3. `password` — the Telegram 2FA password, when enabled;
4. `email_address` or `email_code` — when Telegram requests email verification; when allowed by
   the state, the owner prompt also offers Apple/Google token or an explicit email-code resend;
5. `registration` — only when creating a new account; the CLI shows the ToS, requires explicit
   acceptance, and separately asks whether contacts should be notified (default: no);
6. `ready` — the final state after `getMe` and expected identity verification.

In machine/operator flow, fetch status again after every secret input. Do not assume that OTP is always the next step or that `login_submitted` means success.

The normal interactive loop re-reads state and requests subsequent values:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli login
```

TDLib's `timeout` field is not a promised lifetime for the OTP itself. It specifies when `resendAuthenticationCode` becomes available, and resend also requires a non-null `next_type`. The daemon therefore tracks when it observed the current code challenge. After the timeout, the normal human flow makes one automatic resend request, waits for a new challenge, and only then asks for the fresh code. If Telegram rejects a code before the timeout, the CLI asks for another entry; once the timeout has elapsed, it first attempts to request a new code.

The legacy `login tty` alias remains for compatibility. Use the interactive form only in a real terminal. In CI, a pipe, or a session without `/dev/tty`, protected input must fail instead of falling back to stdin.

## 5. QR login

In `phone_number`/`premium_purchase`, the CLI offers QR instead of a phone number and sends a
typed `requestQrCodeAuthentication`. In `qr_code`, the owner-only link is printed only to
`/dev/tty`, never machine output/MCP. Open it on an already authorized device and wait for
`ready`.

## 6. Confirm completion

The final response must have `status: "ok"`, `state: "ready"`, `challenge_id: null`, and `next_action: "ready"`:

```json
{
  "version": 4,
  "status": "ok",
  "data": {
    "type": "login_status",
    "state": "ready",
    "challenge_id": null,
    "next_action": "ready"
  }
}
```

The daemon publishes `ready` only after TDLib `authorizationStateReady`, a successful `getMe`,
and expected account identity verification. Auth loss revokes current leases; a new Ready repeats
the identity proof.

## 7. Verify returning authorization

Release leases, allow a graceful `close`, and then start the same profile again. It should return to `ready` without another phone/OTP input. Do not call `logOut` or `destroy` for a normal shutdown.

## Troubleshooting

- `secure_tty_unavailable` or `secure_tty_failed` — the command could not obtain a safe `/dev/tty`; open a real interactive terminal;
- stale/invalid challenge — run `login` again, use the current token, and do not reuse the old secret;
- `login_code_resend_unavailable` means the timeout has not elapsed or Telegram supplied no `next_type`; the current code can still be entered;
- `login_code_resend_rejected` means the new-code request reached TDLib but Telegram rejected it; do not retry it in a loop;
- wrong database key — stop the daemon and restore the correct key reference outside model-visible channels;
- `socket_unavailable` — start the daemon with the same `TELEGRAM_PROFILE`;
- `runtime_unavailable` — authorization is not ready for the requested operation;
- `unknown`, an unexpected transition, or recurring `partial` — stop and retain only sanitized state metadata for diagnosis; never copy secret input.

## Current acceptance boundary

The brokered phone/OTP/2FA flow is covered by contract tests and verified live: first login reached `ready`, the daemon closed gracefully, and the same encrypted profile returned to `ready` without another phone/OTP prompt. Actual expired-code resend remains a separate live follow-up; the remaining overall P10 scenarios are also incomplete.
