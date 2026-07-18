# P10 owner TTY failure checkpoint

Date: 2026-07-18 (Europe/Moscow)

## Sanitized live evidence

- The owner ran the exact one-shot command for profile `p10-auth-20260718` and challenge `1` in the Codex app terminal.
- CLI rendered `Телефон:`, then returned closed client error `SecureTtyFailed` before accepting input.
- A fresh broker status after the failure remained `state=phone_number`, `challenge_id=1`, `next_action=submit_via_protected_channel`; no authorization input reached the daemon.
- No phone, OTP, 2FA password, identity, or key value was captured.

## Local correction

- The immediate failure was localized to the post-prompt nonblocking `/dev/tty` wait/read path. The implementation no longer depends on `poll` terminal event compatibility; it performs bounded nonblocking read/retry with signal checks and still has no stdin fallback.
- A delayed nonblocking-input regression test was added. `cargo test --locked -p telegram-cli` passed 6/6, `cargo clippy --locked -p telegram-cli -- -D warnings` passed, and the regular local arm64 CLI binary was rebuilt.
- Live owner retry of the same current challenge remains required before this problem can be resolved.
