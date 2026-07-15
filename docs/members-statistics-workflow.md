# Members и statistics workflows

`supergroup_members` читает reducer-owned `supergroupFullInfo.can_get_members` до
dispatch. Отсутствующий full info означает missing prerequisite, `false` — отдельный
capability denial. `getSupergroupMembers` вызывается с offset и limit `1..=200`; короткая
страница продолжает цепочку. Terminal proof — requested count или `offset >= total_count`.
Пустая страница до объявленного total даёт `NoProgress` и `complete=false`.

`chat_statistics` сначала находит supergroup через reducer-owned chat и проверяет
`supergroupFullInfo.can_get_statistics`. Overview из `getChatStatistics` обходится
рекурсивно: каждый `statisticalGraphAsync` раскрывается через `getStatisticalGraph` до
`statisticalGraphData` или `statisticalGraphError`. Result хранит token lineage. Повтор
token или deadline оставляет исходный async graph и перечисляет его token в
`unresolved_tokens`; поэтому результат остаётся partial.

Оба результата содержат sequence capability-снимка, `observed_at` и
`Freshness::ServerSnapshot`. `observed_at` — локальное время получения workflow result,
а не время Telegram event. `ServerSnapshot` может отставать от реального состояния;
`complete` доказывает только завершение request chain по method-specific правилу.

Workflow не запускают resource optimization, export/cache subsystem или implicit
membership. Opaque graph tokens остаются только в structured result и не логируются.
