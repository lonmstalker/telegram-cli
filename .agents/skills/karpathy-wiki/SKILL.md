---
name: karpathy-wiki
description: "Ведение repo-local Karpathy-style memory: immutable source digests, компактная wiki и раздельные append-only журналы работы, решений и проблем с ротацией. Используй для нетривиальных задач, checkpoints, долговечных discoveries, принятых решений, найденных проблем и обновления project memory."
---

# Karpathy Wiki

Поддерживай source-backed память проекта без transcript dump и секретов.

## Read path

1. Начни с `.memory/wiki/index.md`.
2. Открой только связанные topic/raw pages и нужный хвост активного журнала.
3. Считай live repo/runtime более свежим источником. При конфликте добавь correction entry и обнови synthesis.

## Layout

- `.memory/raw/` — write-once sanitized source digests.
- `.memory/wiki/index.md` — первая страница; `.memory/wiki/*.md` — компактный изменяемый synthesis.
- `.memory/logs/work.md` — действия и checkpoints.
- `.memory/decisions/decisions.md` — принятые, отклонённые и superseded решения.
- `.memory/problems/problems.md` — обнаружение, проверка и закрытие проблем.
- `archive/` рядом с каждым журналом — checksum-indexed immutable shards.

Не дублируй тело решения или проблемы в work log: укажи `D-*` или `P-*` и ссылку.

## Workflow

1. Создай raw digest, только если evidence будет полезен после текущей задачи.
2. Добавь checkpoint в work journal с ID `W-YYYYMMDD-NNN`.
3. Запиши долговечный выбор отдельной decision entry `D-YYYYMMDD-NNN`.
4. Запиши подтверждённую проблему или риск отдельной problem entry `P-YYYYMMDD-NNN`; смену статуса добавляй новой entry с тем же ID.
5. Обнови wiki/index, если изменились canonical facts, active problems или current decisions.
6. Запусти `python3 scripts/rotate-wiki-journal.py --all`; скрипт ничего не меняет, пока ротация не нужна.

## Entry contract

Используй заголовки:

```markdown
## [YYYY-MM-DD] work | W-YYYYMMDD-NNN | Короткий checkpoint
## [YYYY-MM-DD] accepted | D-YYYYMMDD-NNN | Короткое решение
## [YYYY-MM-DD] open | P-YYYYMMDD-NNN | Короткая проблема
```

Work entry содержит цель, sources, actions, verification, ссылки на `D/P` и next. Decision entry содержит context, decision, evidence, consequences и supersedes. Problem entry содержит evidence/reproduction, impact, status, next check/resolution и related decisions.

Не редактируй старые entries. Исправление, supersede или status change оформляй новой записью.

## Rotation

- Active journal ограничен 16 000 Unicode-символов и 1 000 строк.
- Ротируй только целые старейшие entries; сохраняй минимум одну свежую entry active.
- Rotation tool rebases local Markdown links из active directory в `archive/` до checksum и проверяет, что archived targets существуют.
- Не редактируй и не удаляй archive shards и существующие строки archive index.
- `--repair-latest-uncommitted-links` разрешён только для последнего untracked shard и одной final index row, отсутствующей в `HEAD`; tracked immutable shard tool обязан отклонить.
- Проверяй контракт командой `python3 scripts/rotate-wiki-journal.py --all --check`.

## Safety

- Не сохраняй secrets, cookies, tokens, auth files, raw private logs, browser state или raw transcripts.
- Не открывай `.env.local` и не извлекай из него значения. Используй `scripts/with-env-local.sh` и контракт имён из `.env.example`.
- Отделяй source facts от interpretation; помечай assumptions и stale evidence.
