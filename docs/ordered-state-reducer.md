# Ordered state reducer contract

`telegram-core::reducer::StateReducer` применяет `TdJsonEvent::Update` синхронно в том порядке, в котором единственный transport receive loop передал события. Каждый успешно принятый update получает monotonic `UpdateSequence`; то же значение записывается в изменённую cache entry и возвращается caller-у как `AppliedUpdate`.

Unmatched responses и fatal transport events не являются updates и sequence не занимают. Malformed known update fail-closed: cache и global sequence не продвигаются, runtime обязан остановить обычное применение и выполнить явное восстановление, а не продолжать с частичным состоянием.

## Caches

- Authorization хранит последний validated raw `AuthorizationState`.
- User/Chat/BasicGroup/Supergroup/File хранят full raw TDJSON objects с sequence. User/group full-info updates имеют отдельные caches.
- `updateUserStatus` и field-level `updateChat*` изменяют canonical full object только после base update. TDLib guarantee `updateNewChat` используется буквально; partial update без base entity отклоняется. Если nullable TDJSON object field (`last_message`, photo/draft/status/background и аналоги) отсутствует в update, reducer удаляет его из raw cache; отсутствие required field остаётся malformed update.
- Chat positions и membership in chat lists обновляются по exact list discriminator; `order == 0` удаляет position. Reply-markup message сводится к canonical `reply_markup_message_id`; online member count хранится отдельно как transient derived field.
- Connection хранит последний raw `ConnectionState`.
- Message send state индексируется по `(chat_id, old_message_id)` и проходит `Acknowledged -> Succeeded|Failed`; terminal state не может регрессировать.

TDJSON `int53`/`int64` принимаются как strings, которые реально выдаёт pinned ClientJson; numeric form также принимается по его exact input codec. Full raw objects и nested fields не нормализуются сверх перечисленных cache patches.

## Lossless unknown updates

Unknown constructor получает тот же global sequence и сохраняется целиком как raw `Value` в FIFO queue без дедупликации. Read-only slice и ordered drain позволяют runtime переслать payload дальнейшему consumer без изменения fields, nested values или TDJSON integer representation. Field patch известного entity изменяет только перечисленные поля, поэтому неизвестные поля full object сохраняются.

Queue не является durable journal и не заявляет backpressure policy: persistence/limits принадлежат runtime/reliability phases. Gap/resync/freshness также не подменяются обычным sequence.

Behavior tests связывают ordered transport events с reducer sequence и проверяют все перечисленные core cache categories, exact unknown payload/order, chat base-entity gate и terminal message-send transition. Тесты не хранят список/хеш update constructors.
