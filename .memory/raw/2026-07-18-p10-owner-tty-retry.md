# P10 owner TTY retry checkpoint

Date: 2026-07-18 (Europe/Moscow)

## Sanitized live evidence

- After rebuilding the corrected CLI, the owner repeated the same one-shot phone challenge in the Codex app terminal.
- The prompt remained active instead of immediately returning `SecureTtyFailed`. The owner then pressed `Ctrl+C`; CLI restored the terminal and returned the closed cancellation message.
- Fresh broker status remained `state=phone_number`, `challenge_id=1`, `next_action=submit_via_protected_channel`; no secret was submitted.
- The apparent inability to type was expected echo suppression: secure input displays neither characters nor placeholders.

## UX correction and boundary

- All owner TTY prompts now state that input is hidden and must be completed with Enter.
- Fresh `telegram-cli` verification after the prompt change: 6 tests passed, clippy with `-D warnings` passed, and the regular local arm64 binary built successfully.
- Immediate TTY failure P-20260718-001 is resolved. P10 remains pending until the owner types the phone value blindly, presses Enter, and the broker observes a new challenge.
