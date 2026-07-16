# F012 bots/testing workflows

`start_bot_and_wait_reply` строит минимальный воспроизводимый user-side test без нового
spec language. До trigger фиксируется ordered reducer sequence. Затем existing `start_bot`
обязан получить terminal send state; только после этого run ждёт incoming
`updateNewMessage`, где sequence новее boundary, chat совпадает и sender — exact bot.

Reply content по умолчанию закрыт: наружу выходят message/chat/sender IDs, content
constructor и число callback buttons, но не text, labels или callback bytes. Timeout при
непрерывном observation window завершает test как `reply_timed_out`; send uncertainty и
update gap не дают pass и оставляют `complete=false`.

`click_bot_callback` принимает recorded message ID и button coordinates. Core извлекает
payload из lossless cached update и единожды вызывает `getCallbackQueryAnswer`; caller не
пишет TDJSON `@type` и не видит data. TDLib `502` — доказанный `bot_timed_out`, transport
deadline — `uncertain`, остальные errors остаются structured failures. Ни один исход не
повторяет callback автоматически.

Run возвращает outbound и reply message IDs, поэтому cleanup может удалить только exact
owned artifacts. Автоматический delete здесь не добавлен: destructive cleanup выполняется
в P10 на disposable bot/chat с owner confirmation. Inline/game/bot-account/managed-bot
семейства доступны через generated raw registry и остаются default-deny до фактического
consumer; Q001 declarative JSON/YAML spec не решается спекулятивно.

Synthetic runtime proof покрывает terminal start, explicit reply correlation, content
redaction, recorded-button lookup, answered callback и bot timeout. Live bot не вызывался.
