# Phone login review

Fable medium (`claude-fable-5-1`) reviewed the diff and auth coordinator/CLI source,
without tools or real account data. Concrete corrections: bounded status polling,
retry only a definite stale challenge, validate the submission receipt, tolerate daemon
close/restart and another client's lazy start, share the phone prompt, prove agent-mode
rejection leaves the fixture unchanged. Uncertain mutations remain unreplayed.

The native state and the last observed state can briefly differ if a person simultaneously
accepts the old QR. Cancellation applies to that pending login attempt; no atomic native
"log out only if QR" API exists. The UI/documentation do not promise stronger atomicity.
Private socket ownership is the existing same-UID boundary; the owner-TTY restriction is
enforced by the CLI, not a new server-side caller identity system.

The fake daemon tests the CLI's observable cancellation/restart/input behavior; it does not
prove native deletion/closure. Existing daemon lifecycle tests cover external logout → Closed.
Pinned TDLib AuthManager is the source for QR persistence and unauthenticated logOut behavior.

Usage: input 2; cache creation 30 675; cache read 2 061; output 9 608
(thinking 6 544 included). Total 42 346 tokens. CLI list-price estimate $1.09443525;
this is not subscription usage remaining.

Checks: scripts/check.py verify, release build, scripts/test-install.py. Owner phone/OTP
entry and completion of the real account login remain outside these deterministic checks.
