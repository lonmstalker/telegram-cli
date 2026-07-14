# Sanitized source digest: Telegram CLI bootstrap

Дата: 2026-07-15

## Sources

- Явные инструкции пользователя в текущей задаче.
- `product.md`, `plans.md`, `HARNESS.md`, `docs/tdlib-api-coverage.md`.
- Repo-local Karpathy Wiki pattern из `tg-analytics` и `my-harness`; перенесена структура, а не их продуктовые записи.

## Source facts

- Названные Telegram-сценарии являются приоритетными примерами, а не границей поддержки.
- План требует schema-driven coverage всех functions, objects, updates и authorization states закреплённого TDLib.
- Один daemon должен владеть TDLib DB, а несколько агентов — использовать общую сессию через leases.
- CLI обязателен; MCP опционален и не создаёт отдельную TDLib-сессию.
- Пользователь запросил repo-local Karpathy Wiki с раздельным хранением work logs, решений и проблем, включая ротацию.
- Пользователь запросил `.env.local`, перенесённый из нужной части `tg-analytics`, с запретом публикации и agent-readable inspection значений.

## Safety boundary

- Digest не содержит значений, хэшей, длин или префиксов секретов.
- Auth/database/payment/Web App secrets не входят в wiki, journals, Git или model-visible output.
