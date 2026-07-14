# Текущее состояние проекта

Последняя проверка: 2026-07-15.

## Verified

- Документационный bootstrap создан: product, living plan, HARNESS, TDLib coverage contract и F001–F022.
- Pinned planning baseline описывает TDLib 1.8.66: 1010 functions, 2168 definitions, 184 updates и 13 authorization states.
- Existing encrypted TDLib session ранее достигла Ready/getMe и была закрыта через authorizationStateClosed.
- SSH-доступ и серверный database-key path проверены без вывода значения.
- `.env.local` создан как ignored mode-`0600` source; env contract опубликован без значений, loader проверен.
- Karpathy Wiki использует отдельные work/decision/problem journals и checksum-backed rotation.
- Canonical GitHub remote: `https://github.com/lonmstalker/telegram-cli.git`; public visibility явно принята пользователем.
- P0 начат: Cargo workspace содержит шесть целевых packages, а dependency/target/default-member boundaries защищены executable contract с negative controls.
- До появления runtime все четыре binary entrypoint fail closed; process guard ограничен timeout и очищает всю отдельную process group.

## Not implemented

- Generated schema registry, singleton daemon, рабочий CLI и MCP ещё не созданы; текущие binaries являются только fail-closed skeleton.
- Stateful request-chain engine, retry/reconciliation, policy, metrics и agent skill остаются планом.

## Active boundary

- Full API означает L0–L2 для всей pinned schema; curated workflows и live proofs учитываются отдельно.
- Секреты находятся вне model-visible interfaces.
- Gateway key wiring остаётся [P-20260715-001](../problems/problems.md).

## Evidence

- [Bootstrap digest](../raw/2026-07-15-project-bootstrap.md)
- [D-20260715-001](../decisions/decisions.md)
- [D-20260715-002](../decisions/decisions.md)
- [W-20260715-005](../logs/work.md)
