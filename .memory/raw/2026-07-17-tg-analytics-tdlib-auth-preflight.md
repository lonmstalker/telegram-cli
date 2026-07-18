# `tg-analytics` TDLib authorization preflight

Дата: 2026-07-17. External live evidence для P10 authorization. Digest не содержит phone number, API credentials, database key, session contents, DB path или Telegram identity.

## Safe path

- Перед запуском не было процессов `telegram-agent-gateway`, `telegram-agent-cli`, `tg-agent.sh` или `telegramd`, удерживающих TDLib session.
- Использован canonical read-only wrapper source repo: `scripts/tg-agent.sh me`. Он выбирает production profile и удаляет phone-auth variables, поэтому не инициирует fresh login.

## Result

- Gateway завершился с exit code `1` и typed error class `Telegram::Unauthorized`: `Wrong database encryption key`.
- `Ready` и `getMe` не достигнуты; phone number не получен и не выводился.
- После ошибки gateway process отсутствует и DB не удерживается.

## Boundary

- По `tg-analytics` `tdlib-live` guardrail wrong-key является stop signal. Key repair, reauthorization и повторный запуск не выполнялись.
- Эта source session не может служить доказательством P10 first login или безопасно поставлять phone input.
- Локальная returning session текущего проекта отдельно доказана через singleton `telegramd`; её phone field остаётся protected и не передаётся через model-visible output.
