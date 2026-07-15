# TDLib 1.8.66 strict schema parser and inventory digest

Date: 2026-07-15

## Scope

- Input: exact vendored `vendor/tdlib/td_api.tl` from commit `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- Implementation: `crates/telegram-core/src/schema.rs` and its unit tests.
- Boundary: parser/inventory only. Feature-owner classification, capability/risk/retry policy, generated registry, codecs, routers, handlers and runtime are not implemented by this checkpoint.

## Verified model

- Type definitions before `---functions---`: 2168.
- Builtins: 9; object constructors: 2159.
- Methods after `---functions---`: 1010.
- Distinct constructor result type families: 745.
- Update constructors: 184; authorization-state constructors: 13.
- Inventory derives updates/auth states only from the definition section, so methods `testUseUpdate` and `getAuthorizationState` cannot contaminate those sets.
- Source order, source start line, raw documentation, structured tags and canonical signatures are retained; inventory name projections are lexicographically sorted.

## Grammar and resource boundary

- The implementation parses the strict TDLib subset in this pin, not general MTProto TL.
- Supported special declarations are the exact nine builtins, including `double/string ?` and `vector {t:Type} # [ t ] = Vector t`.
- Flags, conditional fields, namespaces, explicit constructor IDs, decorated/generalized parameters, invalid lexical roles, reserved result tokens and ill-kinded `vector`/`Vector` uses fail closed.
- Lowercase concrete constructor references remain valid because the official schema uses them extensively, for example `message:message`, `mask_position:maskPosition`, `proxy:proxy` and `error:error`.
- Direct parser input is capped at 2 MiB before Parser allocation; nested type depth is capped at 32. Parser starts no process, uses no filesystem/network and adds no third-party dependency or Cargo target.

## TDD and review evidence

- Initial consumer test failed because `telegram_protocol::schema` did not exist; architecture review then moved the parser to `telegram-core` and unit tests inside its existing `lib` target.
- Green suite: 12 unit tests, including full pinned corpus and negative controls for structural ambiguity, unsupported syntax, duplicate fields/names, unresolved types, lexical roles, reserved tokens, vector arity, deep nesting and oversized input.
- Independent review found and caused correction of two P1 gaps (lexical roles/reserved tokens; vector arity/kind) and one P2 resource gap (direct input cap).
- Final independent re-review: Approved, no P0/P1/P2 findings. Reviewer re-ran 12 tests, Clippy `-D warnings`, format/diff checks, 11 historical probes and positive bare-constructor/nested-vector controls.

## Reuse audit

- `tg-analytics` contains no reusable Rust TL parser: only an older schema snapshot/update script, request-name substring test and unrelated TDJSON linker/runtime code.
- No NATS, Postgres or analytics orchestration was copied.
