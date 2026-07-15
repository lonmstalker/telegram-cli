# P1 core runtime acceptance digest — 2026-07-15

Sanitized immutable evidence for external native/runtime boundaries. Secret values, account identity and local secret/DB paths are intentionally omitted.

## Artifact identity

- Target: `aarch64-apple-darwin`
- TDLib: `1.8.66`
- Commit: `07d3a0973f5113b0827a04d54a93aaaa9e288348`
- Pinned dylib SHA-256: `5dbd30094b4fbfd35904e88d88e413f423ea7283bd81b34305eac31be6852e7e`

## Native startup handshake

Command shape:

```text
TDJSON_LIBRARY_PATH=<pinned-dylib> cargo test -p telegram-core pinned_native_no_client_call_uses_real_tdjson_transport -- --ignored --exact
```

Result: pass. First request disabled TDLib logging; runtime options matched pinned version/commit; `getCurrentState` produced `authorizationStateWaitTdlibParameters` before any database parameters. The same no-client run sent the secret-shaped canary only after logging was disabled.

## Wrong and missing key boundary

Command shape:

```text
TDJSON_LIBRARY_PATH=<pinned-dylib> cargo test -p telegram-core pinned_native_wrong_or_missing_database_key_is_fail_closed -- --ignored --exact
```

Result: pass against an isolated synthetic test-DC database. Correct synthetic key created DB and normal `close` reached `authorizationStateClosed`. Reopen with another synthetic key returned exact error code 401, never reached `authorizationStateWaitPhoneNumber`, and the top-level DB byte snapshot stayed equal. Missing key file failed before TDLib. Temporary artifacts were deleted.

## Returning live session

Command shape:

```text
TELEGRAM_CORE_LIVE_RETURNING=1 TDJSON_LIBRARY_PATH=<pinned-dylib> scripts/with-env-local.sh -- cargo test -p telegram-core live_returning_session_reaches_ready_without_login_input -- --ignored --exact
```

Result: pass. Protected loader supplied references without environment dump. Existing encrypted production session followed `WaitTdlibParameters -> Ready -> getMe(user) -> close -> Closed`; no phone, OTP or 2FA input was requested. Test output contained neither account identity nor secret values.

## Secret-output canary

Command:

```text
python3 scripts/check-core-secret-output.py
```

Result: `core secret output: ok (native canary clean, negative_controls=1)`. The checker captured both output streams of a pinned-native secret-shaped request after `setLogStream(logStreamEmpty)` and found no canary. Its internal negative control proved the scanner rejects a captured canary.
