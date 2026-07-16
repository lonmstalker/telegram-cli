# CLI schema discovery и universal TDLib call

Второй P6 CLI slice проводит P3 discovery/raw API через тот же private daemon protocol:

```text
telegram-cli schema version
telegram-cli schema capabilities
telegram-cli schema search <terms...>
telegram-cli schema describe <exact-name>
telegram-cli td preview '<tdjson-request>'
telegram-cli td call <lease_id> '<tdjson-request>' ['<approval-json>']
```

CLI не содержит registry и TDLib bindings. Daemon вызывает единственные
`telegram_core::raw_api::{version, capabilities, schema_search, schema_describe, td_call}`.
Descriptors сериализуются из generated data; search не хранит отдельный command matrix.

`td call` принимает один arbitrary pinned-schema request, получает policy только из
matching principal/lease и использует regular-account owner context. Validation,
default-deny/account/risk/approval и transport errors возвращаются закрытыми protocol
codes. Поэтому любой из 1010 methods достигает общего validator/policy gate, но отсутствие
review/scope/approval остаётся честным denial, а не отсутствием CLI API.

Для `admin/destructive/financial/auth_security` route сначала вызывается `td preview`.
Ответ содержит method, risk, retry и exact `plan_hash`; values request не дублируются в
output. Внешний signer возвращает short-lived one-shot JSON receipt с `plan_hash`,
`expires_at_unix`, hex `nonce` и `signature`. Daemon проверяет receipt своим configured
public key и прикрепляет capability только к тому же schema-validated request. Signing key
не существует в CLI, daemon config или model-visible input.

`@type` требуется только в этом explicit raw escape hatch, потому что это TDJSON wire
discriminator. Curated workflow/session команды формируют TDJSON внутри daemon/core и не
требуют его от пользователя. Auth secrets через raw argument запрещены; protected login
route появляется отдельным P6 подпунктом.
