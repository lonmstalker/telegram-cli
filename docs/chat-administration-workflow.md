# F011 groups/channels/moderation

Read resolve и membership остаются разными existing workflows: `resolve_chat` никогда не
join-ит, `ensure_membership` сохраняет `request_pending` как незавершённое состояние, а
`supergroup_members` требует fresh `can_get_members` и не считает short page terminal.
Explicit `leave_chat` использует reviewed `reversible_mutation/reconcile`, ждёт ordered
`chatMemberStatusLeft/Banned` и не повторяет uncertain dispatch.

`plan_chat_title` — bounded configuration planner для primary harness scenario. Он требует
fresh reducer state без gap, cached chat и текущее `permissions.can_change_info`, сохраняет
current/desired title и sequence, затем публикует generated risk/retry и exact plan hash
`setChatTitle`. Title input — обычная Rust string; TDJSON `@type` формируется внутри core.

`apply_chat_title` заново проверяет sequence, current title, право и hash. Изменившийся
snapshot даёт stale-plan failure до dispatch. Уже достигнутый title не требует approval;
реальная mutation принимает только внешний one-shot receipt, ждёт newer matching
`updateChatTitle` и при deadline возвращает `uncertain/complete=false` без blind retry.

Остальная moderation/invite/admin schema доступна через один generated raw route. Опасный
raw method проходит `td preview -> external approval -> td call`; default-deny сохраняется
до точечного capability review. Это оставляет полный pinned API достижимым без второго
слоя per-method Rust wrappers.

Behavior proof покрывает exact approval и matching update, stale/gap rights, pending invite,
members no-progress и forged/replayed receipt. Live configuration/moderation не выполнялись:
P10 использует disposable target и требует owner confirmation перед destructive action.
