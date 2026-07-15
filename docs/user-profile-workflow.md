# F007 user/profile workflow

P7 добавляет два curated route поверх generated raw API:

- `user_profile` принимает `self`, numeric user ID или public username. Cache miss вызывает
  `getUser` либо `searchPublicChat`; username обязан разрешиться в private chat, после чего
  optional `getUserFullInfo` использует найденный user ID.
- `update_profile_name` сначала читает `getMe`, не dispatch-ит уже достигнутое состояние,
  затем вызывает `setName` и принимает success только после ordered `updateUser` с exact
  first/last name. Deadline после accepted request возвращает `uncertain`, не blind retry.

Оба route блокируются на update gap. Profile view возвращает только selected public/state
fields. Phone, birthdate, private note и business info никогда не сериализуются: для них
публикуется только closed `unavailable|redacted`. Отсутствующий private field не делает
самого user incomplete и не превращается в выдуманное значение.

Capability data ревьюит только фактических consumers: `getMe`/`getUser` как safe reads и
`setName` как convergent reversible mutation; existing `searchPublicChat` и
`getUserFullInfo` contracts переиспользуются. Остальные contact/profile methods остаются
достижимы через universal raw gate, но default-deny до отдельного runtime contract — это
валидное состояние по `plans.md`.

Behavior test доказывает resolver-before-result, ordered cache application, private-field
canary redaction и verified name transition. Live profile/contact mutation реального
аккаунта не выполнялась; это P10/disposable-fixture boundary.
