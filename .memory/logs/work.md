# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

## [2026-07-15] work | W-20260715-001 | Bootstrap Karpathy Wiki и secret-safe env

- Цель: добавить repo-local Karpathy Wiki, правила ротации, отдельные decision/problem journals и безопасный `.env.local` workflow.
- Sources: инструкция пользователя, `product.md`, `plans.md`, `HARNESS.md`, source digest `../raw/2026-07-15-project-bootstrap.md`, patterns из `tg-analytics` и `my-harness`.
- Actions: инициализирован `.agents/skills/karpathy-wiki`; выбран generic three-journal rotation contract; выполнена secret-safe инвентаризация нужных env names без вывода значений.
- Verification: skill/script/env checks ещё не завершены на этом checkpoint.
- Decisions: [D-20260715-001](../decisions/decisions.md).
- Problems: [P-20260715-001](../problems/problems.md).
- Next: создать `.env.local` атомарным quiet transfer, проверить loader, skill, journals, ссылки, permissions и Git ignore.

## [2026-07-15] work | W-20260715-002 | Wiki и local env bootstrap проверены

- Цель: закрыть bootstrap без раскрытия env values и подтвердить рабочую ротацию трёх журналов.
- Sources: `.agents/skills/karpathy-wiki`, `AGENTS.md`, `.env.example`, `scripts/with-env-local.sh`, `scripts/rotate-wiki-journal.py`.
- Actions: создан `.env.local` с минимальными TDLib development entries; irrelevant bot/admin/database vars и phone не переносились; добавлены protected loader и Git ignore.
- Verification: skill validator passed; loader подтвердил обязательные значения и file/dir references без вывода; mode `0600` и Git ignore подтверждены; work/decision/problem contracts passed; synthetic rotation создала checksum-indexed shard из целой entry.
- Decisions: [D-20260715-001](../decisions/decisions.md).
- Problems: [P-20260715-001](../problems/problems.md) остаётся open до реализации gateway key provider.
- Next: использовать wiki при P0/P1, а `.env.local` — только через protected loader.
