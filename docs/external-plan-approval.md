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

## Durable boundary

Approval разрешает dispatch, но не заменяет idempotency: execution сначала сохраняет
operation `pending` в [durable journal](idempotency-journal.md). После crash exact operation
становится `uncertain` и требует reconciliation, поэтому повторно переданная capability не
разрешает blind duplicate. Wire receipt transport и protected operator UI принадлежат P6;
P5 фиксирует verifier/policy trust boundary.
