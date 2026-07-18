# Telegram CLI User Guide

[Русская версия](user-guide.ru.md) · [Step-by-step authorization](authorization-guide.en.md)

## Guide status

This guide covers running from a source checkout. Packaged installation, systemd/launchd integration, and upgrades belong to P9 and are not yet claimed as complete. First authorization with phone/OTP/2FA and restarting the same encrypted profile have been verified live; the remaining P10 scenarios are not complete yet.

## What runs

`telegramd` is the only process that owns the TDLib database for a selected profile. `telegram-cli` connects to its private Unix socket and never opens the database directly.

Key terms:

- **profile** — local configuration and a separate TDLib session;
- **daemon** — the `telegramd` process that owns the session;
- **lease** — a time-limited permission for operations with specific scopes;
- **workflow** — a reviewed higher-level operation;
- **raw call** — a direct TDLib request that still passes generated validation and default-deny policy.

## 1. Prepare the checkout

You need a Rust toolchain and a pinned native `tdjson` artifact matching the current checkout. Build the applications from the repository root:

```sh
cargo build --locked -p telegramd -p telegram-cli
```

If `.env.local` does not exist yet, create it from the example and immediately restrict its permissions:

```sh
cp .env.example .env.local
chmod 600 .env.local
```

Do not overwrite an existing `.env.local`. Fill in the settings described in `.env.example`. In particular, you need Telegram API credentials, profile paths, and TDLib database paths. The file referenced by `TDLIB_DATABASE_KEY_FILE` must contain the Base64-encoded database key and have mode `0600`.

Phone, OTP, 2FA password, email code, and registration data must not be placed in `.env.local`, command arguments, stdin, or logs. Enter them only through the protected TTY flow.

## 2. Start the daemon

In the first terminal:

```sh
scripts/with-env-local.sh -- target/debug/telegramd
```

Keep the process running. The daemon automatically begins a graceful shutdown after a period with no leases or active work.

## 3. Check authorization

In the second terminal:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli login
```

The CLI sequentially requests phone, OTP, 2FA/cloud password, and any additional fields required by Telegram. For a stale code challenge, it requests a new code after the server-specified timeout when TDLib offers another delivery method. The command exits only after proven `ready` or an explicit error/cancellation. All fields are visible by default; before cloud password the CLI separately offers to hide echo.

Use the separate machine-status form when no interactive prompts are allowed:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli --output json login
```

In machine flow, continue only when the root `status` is `ok` and the login `state` is `ready`. See the [authorization guide](authorization-guide.en.md) for details and operator/MCP handoff.

## 4. Run the first read operation

Acquire a read lease:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli --output json session hold read 60000
```

Save the `lease_id` from the response. You can inspect the current workflow contract before running it:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli --output json workflow describe user_profile
```

Fetch the current user's profile, replacing `LEASE_ID` with your value:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli --output json workflow run LEASE_ID user_profile '{"target":{"kind":"self"},"include_full_info":true}'
```

Release the lease when finished:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli --output json session release LEASE_ID
```

## 5. Interpret machine output correctly

The JSON envelope has `version: 3` and a root `status`:

- `ok` — the operation completed with the stated result;
- `partial` — the result is incomplete or requires reconciliation; this is not success;
- `error` — the request was rejected or not executed.

A zero exit code alone does not replace checking `status`, `complete`, `next_action`, and other response-specific fields. Do not blindly retry a mutation after an uncertain outcome.

## Useful commands

```sh
# Daemon and lease status
scripts/with-env-local.sh -- target/debug/telegram-cli --output json status

# List and describe workflows
scripts/with-env-local.sh -- target/debug/telegram-cli --output json workflow list
scripts/with-env-local.sh -- target/debug/telegram-cli --output json workflow describe WORKFLOW

# Inspect the pinned schema
scripts/with-env-local.sh -- target/debug/telegram-cli --output json schema version
scripts/with-env-local.sh -- target/debug/telegram-cli --output json schema search QUERY
scripts/with-env-local.sh -- target/debug/telegram-cli --output json schema describe TD_METHOD

# Preview a direct request before sending it
scripts/with-env-local.sh -- target/debug/telegram-cli --output json td preview '{"@type":"getMe"}'
```

Prefer curated workflows for normal use. A raw call requires a lease, an exact `@type`, and may be rejected by policy even when the TDLib method is valid.

## Safe shutdown

Release leases first. When no work remains, the daemon calls `close`, preserves authorization, and exits. Do not use `logOut` or `destroy` for a normal shutdown: both are destructive operations.

## Common errors

- `socket_unavailable` — the daemon is not running, a different profile is selected, or the private socket is unavailable;
- `runtime_unavailable` — the TDLib runtime has not reached the state required by the command;
- wrong database key — stop the daemon and fix the reference to the correct key; do not start a new phone authorization over an existing database;
- `partial` — perform the stated `next_action` or reconciliation and do not treat the response as complete;
- lease expired — acquire a new lease and re-check state before a mutation.

Technical contracts live in [`docs/`](.). Start with the [CLI session contract](cli-session.md), [workflow routes](cli-workflows.md), and [secure login contract](cli-secure-login.md).
