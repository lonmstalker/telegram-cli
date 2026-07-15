# Core generated raw API

`telegram_core::raw_api` — одна discovery/call поверхность поверх exact generated
registry и daemon-owned `CoreRuntime`.

- `version(runtime)` возвращает verified runtime TDLib version/commit и pinned schema
  SHA-256. `CoreRuntime` уже остановил startup при mismatch.
- `capabilities()` возвращает generated disposition каждого pinned method, включая
  `DefaultDeny`.
- `schema_search(query)` применяет case-insensitive AND по whitespace tokens к exact
  name/signature/documentation и type names; результат deterministic sorted.
- `schema_describe(name)` возвращает exact method/constructor/builtin descriptor либо
  type family со всеми constructors.
- `td_call(runtime, value, deadline)` рекурсивно валидирует request по registry,
  использует существующий transport-owned `@extra`, принимает TDLib `error` как raw
  result, проверяет known successful result family и сохраняет неизвестный future
  object losslessly.

Per-method wrappers и application-level literals `@type` не нужны: caller выбирает
method через discovery, а discriminator строит общий JSON contract. На этом checkpoint
product binaries ещё не публикуют raw dispatch. Обязательный policy check внутри
`td_call` добавляет следующий Tasks-пункт P3 до daemon/CLI wiring.
