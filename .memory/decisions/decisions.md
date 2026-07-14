# Decision Journal

Active append-only decision records. Изменение решения оформляется новой entry; старое решение не переписывается.

## [2026-07-15] accepted | D-20260715-001 | Раздельная memory model и secret boundary

- Context: исходный Karpathy Wiki объединял действия, решения и проблемы в одном log; пользователь потребовал раздельное хранение и ротацию.
- Decision: хранить work, decisions и problems в трёх active journals с отдельными checksum archives; raw digests оставить immutable, wiki — compact synthesis.
- Rotation: active journal ограничен 16 000 Unicode-символов и 1 000 строк; generic script переносит только целые старейшие entries и оставляет минимум одну свежую.
- Secret boundary: `.env.local` является canonical local secret source, используется только через `scripts/with-env-local.sh`, никогда не читается или логируется агентом.
- Evidence: [bootstrap digest](../raw/2026-07-15-project-bootstrap.md), явная инструкция пользователя, repo patterns `tg-analytics` и `my-harness`.
- Alternatives: один общий log отклонён из-за смешения concerns; ручная monthly rotation отклонена из-за отсутствия checksum/index verification.
- Consequences: каждый checkpoint должен ссылаться на `D/P` IDs; archive shards и index rows immutable.
- Supersedes: none.

## [2026-07-15] accepted | D-20260715-002 | Публичный GitHub remote

- Context: первоначально запрашивался private remote, но созданный пользователем `lonmstalker/telegram-cli` имеет visibility `PUBLIC`.
- Decision: сохранить репозиторий публичным и использовать его как canonical `origin`.
- Evidence: явное подтверждение пользователя «он публичный, это ок»; `gh repo view lonmstalker/telegram-cli --json nameWithOwner,visibility,url,defaultBranchRef` вернул `PUBLIC`.
- Alternatives: изменение visibility на `PRIVATE` отклонено пользователем как ненужное.
- Consequences: в Git можно публиковать только sanitized artifacts; `.env.local` и другие secrets обязаны оставаться ignored и untracked.
- Supersedes: первоначальное требование private visibility из запроса на создание remote.

## [2026-07-15] accepted | D-20260715-003 | Exact initial production schema pin

- Context: F003/Q001 требовал выбрать initial production TDLib schema identity до parser/runtime implementation; moving `master` не даёт воспроизводимости.
- Decision: закрепить TDLib `1.8.66` exact commit `07d3a0973f5113b0827a04d54a93aaaa9e288348`; manifest и vendored `td_api.tl` являются repository source of truth, а обновление требует явного reviewed diff.
- Evidence: [immutable source digest](../raw/2026-07-15-tdlib-1.8.66-schema-pin.md), `vendor/tdlib/manifest.json`, `python3 scripts/check-tdlib-pin.py`.
- Alternatives: следование moving `master` отклонено как невоспроизводимое; внешний generated snapshot отклонён без повторной exact provenance-проверки.
- Consequences: parser, registry и runtime handshake используют этот commit/hash; native artifact остаётся отдельным незакрытым P0 proof.
- Supersedes: none.
