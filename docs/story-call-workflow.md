# Story и group-call lifecycle

F015 добавляет четыре typed route без отдельного story/call framework:

```text
plan_story_mutation     apply_story_mutation
inspect_group_call      leave_group_call
```

Story input имеет `action=post_photo|delete`. Photo post принимает только file ID после
terminal `upload_sticker_file`/F010 upload; core заново проверяет
`remote.is_uploading_completed` и сам строит `inputStoryContentPhoto`, `formattedText` и
privacy constructors. Privacy — closed enum `everyone|contacts|close_friends|selected_users`.

Post classified `admin`, delete — `destructive`; обе action требуют external one-shot
approval exact request hash. Успешный post не считается published по временному response:
core перечитывает exact story и требует `is_being_posted=false`. При потерянном response
workflow один раз читает active-story snapshot, возвращает candidate IDs и остаётся
`uncertain`; повторного post нет.

Delete сначала перечитывает exact story и проверяет `can_be_deleted`. Cleanup complete
только при подтверждённом `deleteStory` и отсутствии ID в fresh active-story snapshot.
Timeout остаётся uncertain даже при отсутствии ID, поскольку expiration и deletion нельзя
смешивать.

`inspect_group_call` возвращает только state flags. `leave_group_call` — desired-state
cleanup: already-left не dispatch-ится, success/timeout всегда завершается fresh
`getGroupCall`; только `is_joined=false` доказывает освобождение ресурса.

Join/video/live payload не является CLI JSON. `groupCallJoinParameters.payload`, join
response и media sockets принадлежат tgcalls adapter и могут содержать signaling material.
Выбор adapter/fixture остаётся harness Q001; F015 не выдаёт acknowledgement за active call
и не изобретает WebRTC transport.

Synthetic test покрывает published reread, lost-response story reconciliation, exact story
cleanup и leave-after-timeout probe. Live story/call действия остаются P10 и требуют
consenting fixture/явного разрешения перед side effect.
