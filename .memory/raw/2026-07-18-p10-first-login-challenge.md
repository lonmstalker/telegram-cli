# P10 first-login challenge checkpoint

Date: 2026-07-18 (Europe/Moscow)

## Scope

- Prepared a new isolated local profile `p10-auth-20260718`; the existing returning TDLib database was not opened or changed.
- `.env.local` was used only through `scripts/with-env-local.sh`; its values were not read or printed.
- Session directories are Git-ignored and mode `0700`. TDLib production DC was selected explicitly.

## Sanitized live evidence

- Before launch, the default CLI profile returned machine envelope v3 `socket_unavailable`; no existing daemon owner was active through that profile socket.
- The new singleton `telegramd` started with pinned TDLib `1.8.66` / commit `07d3a0973f5113b0827a04d54a93aaaa9e288348` and passed from `WaitTdlibParameters` to a brokered phone challenge.
- Fresh CLI status: root `status=partial`, `state=phone_number`, `challenge_id=1`, `next_action=submit_via_protected_channel`.
- No phone, OTP, 2FA password, database key, account identity, or private path value was submitted or captured in this checkpoint.

## Boundary and next action

- The daemon remains the only TDLib database owner for this profile while the owner performs the protected TTY step.
- Owner command: `scripts/with-env-local.sh -- env TELEGRAM_PROFILE=p10-auth-20260718 target/debug/telegram-cli login tty 1`.
- `LoginSubmitted` will not prove completion. After owner input, read a fresh status and follow the new challenge until `Ready + getMe`; then verify a returning restart before accepting P10 authorization.
