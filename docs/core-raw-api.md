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
- `td_call(runtime, policy, value, deadline)` рекурсивно валидирует request по registry,
  до transport требует reviewed capability row, matching trusted account kind и granted
  risk class, затем
  использует существующий transport-owned `@extra`, принимает TDLib `error` как raw
  result, проверяет known successful result family и сохраняет неизвестный future
  object losslessly.

Per-method wrappers и application-level literals `@type` не нужны: caller выбирает
method через discovery, а discriminator строит общий JSON contract. `RawPolicy`
создаётся trusted product layer из account/risk grants; agent input ещё не подключён.
Unreviewed method, wrong account и missing risk возвращают `PolicyError` до send.
Runtime-requirement expression остаётся discoverable prerequisite и не выдаётся за
удовлетворённое без будущего live-state consumer.
