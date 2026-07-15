# Ordered state reducer contract

`telegram-core::reducer::StateReducer` применяет `TdJsonEvent::Update` синхронно в том порядке, в котором единственный transport receive loop передал события. Каждый успешно принятый update получает monotonic `UpdateSequence`; то же значение записывается в изменённую cache entry и возвращается caller-у как `AppliedUpdate`.

Unmatched responses и fatal transport events не являются updates и sequence не занимают. Malformed known update fail-closed: cache и global sequence не продвигаются, runtime обязан остановить обычное применение и выполнить явное восстановление, а не продолжать с частичным состоянием.

## Caches

- Authorization хранит последний validated raw `AuthorizationState`.
- User/Chat/BasicGroup/Supergroup/File хранят full raw TDJSON objects с sequence. User/group full-info updates имеют отдельные caches.
- `updateUserStatus` и field-level `updateChat*` изменяют canonical full object только после base update. TDLib guarantee `updateNewChat` используется буквально; partial update без base entity отклоняется.
- Chat positions и membership in chat lists обновляются по exact list discriminator; `order == 0` удаляет position. Reply-markup message сводится к canonical `reply_markup_message_id`; online member count хранится отдельно как transient derived field.
- Connection хранит последний raw `ConnectionState`.
- Message send state индексируется по `(chat_id, old_message_id)` и проходит `Acknowledged -> Succeeded|Failed`; terminal state не может регрессировать.

TDJSON `int53`/`int64` принимаются как strings, которые реально выдаёт pinned ClientJson; numeric form также принимается по его exact input codec. Full raw objects и nested fields не нормализуются сверх перечисленных cache patches.

## Текущая граница

Unknown constructor получает sequence и outcome `Unknown`, но raw payload пока не сохраняется. Это намеренная граница следующего Tasks-пункта P1, а не claim lossless coverage. Gap/resync/freshness принадлежат дальнейшим фазам и не изобретаются здесь.

Behavior tests связывают ordered transport events с reducer sequence, проверяют все перечисленные core cache categories, chat base-entity gate и terminal message-send transition. Тесты не хранят список/хеш update constructors.
