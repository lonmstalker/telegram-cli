# External exact-plan approval

Опасные raw requests (`admin`, `destructive`, `financial`, `auth_security`) требуют
`ApprovedPlan` дополнительно к account/risk lease policy. Проверка находится внутри общего
`td_call`, поэтому другой raw dispatcher не может обойти gate.

## Preview и подпись

`PlanPreview::for_request` принимает уже schema-validated request и публикует только method,
risk, retry class и domain-separated SHA-256 `PlanHash`. Значения request не печатаются:
external operator/broker обязан показать exact plan через отдельный protected channel и
подписать payload `domain || plan_hash || expires_at || nonce`.

Daemon настраивается optional `TELEGRAM_APPROVAL_PUBLIC_KEY_HEX` и хранит только Ed25519
public key. Signing key не входит в repo, `.env.local`, daemon, CLI/MCP arguments или
model-visible interfaces. При отсутствии public key dangerous request остаётся
`approval_required`.

`ApprovalVerifier` проверяет exact hash, expiry и strict Ed25519 signature. Nonce принимается
один раз за daemon boot; выданный `ApprovedPlan` также consumable ровно одним matching
request. Hash mismatch, expiry, повтор или forged signature fail closed до transport.

## Wire path

`telegram-cli td preview '<request-json>'` получает hash из daemon без TDLib dispatch.
`td call ... '<approval-json>'` и approved workflow передают receipt в stable protocol;
nonce/signature redacted в Debug и zeroized после decode. Daemon хранит verifier весь boot,
а отсутствие configured public key даёт `approval_denied` для предъявленного receipt и
`approval_required` для опасного вызова без receipt.

Workflow approval не подписывает имя route: daemon восстанавливает exact TDLib request из
strict typed input, проверяет тот же plan hash, а core расходует capability только внутри
matching common `td_call`. Этот путь без второго signer/verifier используют
`apply_chat_title`, `apply_custom_emoji_set`, `apply_story_mutation` и
`apply_terminate_session`.

## Durable boundary

Approval разрешает dispatch, но не заменяет idempotency: execution сначала сохраняет
operation `pending` в [durable journal](idempotency-journal.md). После crash exact operation
становится `uncertain` и требует reconciliation, поэтому повторно переданная capability не
разрешает blind duplicate. Protected operator UI остаётся внешним owner-controlled каналом;
CLI/daemon не получают signing key.
