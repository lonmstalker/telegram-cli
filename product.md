# product.md

Product name: Telegram Agent CLI
One-line description: Telegram Agent CLI даёт AI-агентам и операторам полный, безопасный и предсказуемый доступ к Telegram через официальный TDLib.
Product category: Локальная и серверная платформа автоматизации Telegram-аккаунта
Core promise: Любая возможность закреплённой TDLib-схемы доступна агенту через единое ядро, а многошаговые операции возвращают актуальный и честно размеченный результат без дублей сессий.

## Audience

- AI-агенты — собирают статистику, выполняют рутинные операции и тестируют Telegram-продукты.
- Владелец Telegram-аккаунта — делегирует ограниченные действия, сохраняя контроль над секретами и опасными операциями.
- Оператор или deployer — устанавливает платформу локально либо на сервере, наблюдает состояние и восстанавливает работу.
- Разработчик Telegram-ботов и Mini Apps — проверяет account-side сценарии, callbacks и Web App launch вместе с браузерным harness.

## Pains Solved

- Агент получает `not found` для существующего объекта, когда TDLib ещё не выполнил prerequisite-запросы -> платформа продолжает цепочку и явно показывает полноту результата.
- Несколько агентов запускают отдельные TDLib-клиенты и конфликтуют за одну базу -> один broker переиспользует одну сессию на профиль.
- Частичная ручная обёртка быстро отстаёт от TDLib -> полный каталог строится из закреплённого `td_api.tl`.
- CLI, MCP и автоматизации расходятся по семантике -> все поверхности используют один protocol и одно ядро.
- Неясно, можно ли повторить неуспешную мутацию -> результат получает idempotency/reconciliation status, а опасные действия проходят policy gate.

## Core Workflows

- Получить или переиспользовать lease одной авторизованной сессии и корректно освободить его.
- Координировать first login или re-auth через CLI/MCP: агент получает challenge ID и status, а владелец вводит secret вне model-visible transport.
- Найти, описать и вызвать любую функцию закреплённой TDLib-схемы.
- Выполнить stateful read workflow: resolve, preload/open, paginate, дождаться updates, вернуть freshness и completeness.
- Выполнить контролируемую мутацию: прочитать текущее состояние, показать diff, подтвердить, применить и проверить результат.
- Собирать статистику чатов и каналов с честным разделением official, derived, partial и forbidden данных.
- Управлять чатами, каналами, сообщениями, файлами, ботами, sticker/custom emoji packs и другими TDLib-доменами.
- Тестировать ботов и Mini Apps: TDLib управляет Telegram-side flow, браузерный harness проверяет UI, bridge и сеть.

## Product Rules

- Названные пользователем кейсы — приоритетные примеры, а не граница продукта.
- Граница поддержки определяется всеми functions, objects, updates и authorization states закреплённого `td_api.tl`.
- Недоступность метода из-за типа аккаунта, прав, Premium/Business или official-app ограничения не удаляет метод из каталога; она возвращается как capability/policy result.
- На один account profile существует ровно один владелец TDLib DB и одна переиспользуемая сессия.
- `partial`, `pending`, `stale` и `uncertain` не равны доказанному `not_found`.
- CLI обязателен; MCP является опциональным адаптером и не добавляет отдельную бизнес-логику.
- Destructive, account-security и financial API поддерживаются схемой, но могут быть default-deny.

## MVP Vision

Первый production-ready срез включает закреплённую полную TDLib-схему, единый core, singleton daemon и CLI. Агент может обнаружить capability, безопасно вызвать любой метод, использовать curated workflows для зависимых операций и переиспользовать одну сессию вместе с другими агентами. MCP добавляется только после доказанной полноты и устойчивости CLI/core.

## Non-goals

- Не переimplementировать MTProto и не обходить ограничения Telegram.
- Не добавлять stealth, anti-detection, ban-evasion или имитацию человека.
- Не обещать доступ к данным или действиям, которых у аккаунта нет по правам.
- Не передавать API hash, database key, phone, OTP, 2FA, Passport или Web App init data в model context, stdout, logs или metrics.
- Не считать TDLib заменой браузеру для визуального и DOM-тестирования Mini Apps.
- Не публиковать неаутентифицированный remote endpoint.

## Agent Context

- Terms: profile — один аккаунт и его отдельные DB/files; lease — право клиента использовать daemon; workflow — stateful цепочка TDLib-вызовов и updates; raw call — один schema-validated метод; terminal proof — доказательство завершённости результата.
- Surfaces: `telegram-cli`, singleton daemon, опциональный `telegram-mcp`, browser runner.
- Trust boundaries: модель не получает auth/encryption secrets; daemon единолично владеет DB; remote transport всегда аутентифицирован; destructive/financial действия требуют отдельного scope или подтверждения.
