# Platform utilities и proxy transition

F020 не создаёт family-модуль для localization/options/themes/log/test API. Все 1010
методов уже находятся в generated registry; pinned schema hash и default-deny policy
закрывают SC001 без второго классификатора.

Curated proxy surface ограничен routes, которым не нужны endpoint credentials:

```text
proxy_status
set_proxy_enabled
```

`proxy_status` возвращает только TDLib proxy ID, enabled flag и type constructor. Server,
port, comment, username, password и MTProto secret не сериализуются. Add/edit/ping raw
payloads остаются default-deny, пока не появится protected provider.

Setter принимает tagged `enable {proxy_id}` или `disable`; missing action не означает
disable. Он fresh-читает proxy list, сохраняет предыдущий enabled ID как rollback target,
dispatch-ит ровно один `enableProxy|disableProxy`, затем перечитывает list через ordered
runtime boundary. `verified` требует desired proxy state и более новый
`connectionStateReady`. Совпавший proxy state без нового Ready даёт
`connectivity_diverged/complete=false`; response timeout также ведёт только к reread, без
повтора mutation.

Synthetic SC002 моделирует successful setter и последующий non-ready connection, проверяет
rollback ID и отсутствие endpoint canaries. Live proxy/network mutation не выполнялась.
