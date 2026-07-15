# Daemon lease contract

Третий P2 slice реализован типами `telegram-protocol::{LeaseRequest, LeaseResponse}` и `telegramd::{lease, server}`. Lease — временное право principal использовать одну daemon-owned session; это не TDLib request и не заменяет policy authorization.

## JSONL wire surface

Одна Unix connection несёт один JSON object и один response, оба завершаются LF:

- `lease_acquire`: `principal`, список closed `RiskScope`, `ttl_ms`.
- `lease_heartbeat`: `lease_id`, тот же `principal`.
- `lease_release`: `lease_id`, тот же `principal`.
- Responses: `lease_granted`, `lease_renewed`, `lease_released` или `error` со stable code.

Frame ограничен 16 KiB и 5-second client IO timeout, потому что текущий sequential accept loop не должен блокироваться бесконечным local client. Malformed, oversized и незавершённый JSONL frame получает `invalid_request` либо connection error; payload не логируется.

## Identity, scopes и TTL

- Lease ID включает daemon boot epoch и monotonic counter, поэтому stale client ID не совпадает с новым lease после restart. ID является correlation identity, не secret capability.
- Principal обязан быть non-empty и без control characters. Heartbeat/release другого principal возвращает `principal_mismatch`.
- Scopes — non-empty typed values: `read`, `presence`, `send`, `reversible_mutation`,
  `admin`, `destructive`, `financial`, `auth_security`. Unknown value не десериализуется.
- `TELEGRAM_RISK_SCOPES` задаёт owner ceiling; missing/empty configuration означает
  conservative read-only. Lease request за пределами ceiling получает `scope_denied`.
- Действующий lease строит `RawPolicy` только из своих scopes и trusted account kind.
  Generated capability row по-прежнему определяет risk каждого method; caller не передаёт
  method class.
- Requested TTL должен быть `1..=60000` ms. Heartbeat до expiry продлевает lease на исходный TTL; explicit release удаляет его немедленно.
- Expired lease удаляется при lease operation и возвращает `lease_expired`; [lifecycle loop](daemon-session-lifecycle.md) также вызывает background expiry перед каждой idle eligibility check. Поэтому disconnect клиента не удерживает session дольше TTL.

Local socket `0600` ограничивает callers effective user, но principal пока self-asserted. Authenticated remote identity принадлежит optional MCP/server boundary P8; текущий principal match предотвращает accidental cross-client renew/release, а не изображает remote authentication.

## Verification

- Deterministic manager tests подтверждают unique ID, sorted/deduplicated typed scopes,
  owner ceiling, scoped `RawPolicy`, principal match, heartbeat extension, explicit release
  и expiry.
- Real Unix socket test выполняет acquire -> heartbeat -> release через serialized protocol types.
- Process-level synthetic daemon gate подтвердил acquire/normalized scopes, heartbeat, release и `lease_expired` без TDLib или `.env.local`.
- Protected live gate подтвердил, что client disconnect без release удерживает daemon до TTL, после чего zero-activity idle close доходит до `authorizationStateClosed`.
