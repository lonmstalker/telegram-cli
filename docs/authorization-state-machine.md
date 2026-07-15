# Authorization state machine contract

`telegram-core::authorization` обрабатывает pinned TDLib authorization states как один state/challenge machine. Это pure core: он принимает raw `authorizationState*`/`updateAuthorizationState`, возвращает безопасный step и строит exact auth request только после input для текущего challenge.

## Состояния и steps

- `authorizationStateWaitTdlibParameters` → `ParametersRequired`; текущая generation принимает только защищённо загруженный key через `submit_parameters` и exact `setTdlibParameters`.
- `WaitPhoneNumber` → phone/QR challenge.
- `WaitPremiumPurchase` → явный financial challenge без автоматической покупки.
- `WaitEmailAddress`/`WaitEmailCode` → email address, code, allowed Apple ID/Google ID token branches и typed reset timers.
- `WaitCode` → authentication-code challenge с delivery/next type и resend timeout.
- `WaitOtherDeviceConfirmation` → protected QR link; новое update создаёт новый challenge ID.
- `WaitRegistration` → terms summary и bounded first/last name input.
- `WaitPassword` → 2FA challenge с redacted hint/recovery pattern.
- `Ready`, `LoggingOut`, `Closing`, `Closed` → explicit lifecycle steps.

Unknown state/type/required field fail closed. `Ready` означает только наблюдённый TDLib state; login terminal proof остаётся `Ready` плюс успешный `getMe` expected-identity check.

## Input discipline

- Каждое observed state получает monotonic `ChallengeId`; input со старым ID отклоняется.
- После одного принятого input повторная submission блокируется до нового state update либо явного `submission_failed` от runtime driver.
- Phone/QR restart допускается только в состояниях, перечисленных pinned schema.
- Email identity tokens допускаются только при соответствующем `allow_apple_id`/`allow_google_id`.
- Premium purchase, password recovery/reset и destructive account actions не изобретаются этим machine.

`SensitiveString` скрывает значение в `Debug` и zeroizes owned storage on drop. `AuthorizationRequest::Debug` показывает только request type; caller обязан сразу передать `into_value()` transport и не логировать полученный JSON. Protocol challenge IDs/status не содержат secret values.

Database key живёт в отдельном zeroizing wrapper. TDLib error `401` включает fail-closed latch и запрещает переход к phone/QR до явной повторной подачи parameters. Источники и TDJSON codec закреплены в [database-key contract](database-encryption-key.md).

## Verification

Behavior tests проходят phone, QR, code, 2FA, email address/code/Apple token, device refresh, registration, parameters/premium и terminal states. Responses проверяются по exact TDLib request shape; неизвестный state, stale challenge, duplicate submission и mismatched input проверяются негативно. Тесты не хранят schema hash/count и не мутируют pinned schema.
