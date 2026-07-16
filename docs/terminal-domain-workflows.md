# Terminal file, sticker, bot и Web App workflows

Curated Rust API формирует method и nested TDJSON discriminators внутри core:
`DownloadQuery`, `InputFileSource`, `StickerFormat`, `WebAppRequest` и `WebAppMode`
не требуют от caller писать `@type`. Общий raw API остаётся отдельным lossless escape hatch.

`download_file` отправляет asynchronous `downloadFile` и считает transfer завершённым
только когда response уже имеет `local.is_downloading_completed=true` либо matching
ordered `updateFile` после исходной sequence boundary показывает это состояние.
`upload_sticker_file` применяет симметричное правило к
`remote.is_uploading_completed`. Allocated file ID, progress и inactive state сами по
себе не являются success. Если actual/expected size известен, terminal receipt дополнительно
проверяет downloaded/uploaded bytes; mismatch отклоняется, `size_verified` показывает proof.
`cancel_download` сначала читает `getFile`, не dispatch-ит уже остановленный transfer и
после request/timeout снова проверяет `is_downloading_active`; mismatch остаётся uncertain.

`start_bot` сохраняет temporary message ID из `sendBotStartMessage`, затем ждёт
`updateMessageSendSucceeded` или `updateMessageSendFailed` через reducer-owned
`MessageSendState`. Acknowledgement не terminal. Deadline даёт `Uncertain` и запрещает
blind repeat; workflow не утверждает, что бот ответил — reply correlation принадлежит
будущему bot-test orchestration.

F012 добавляет orchestration без отдельного test framework. `start_bot_and_wait_reply`
фиксирует reducer sequence до trigger, переиспользует terminal-correct `start_bot` и
принимает только subsequent incoming `updateNewMessage` от exact bot в exact chat. Result
содержит message ID, content constructor и число callback buttons; text/button labels и
payload redacted. Reply deadline — terminal failed test, send uncertainty или update gap —
`complete=false` без blind start.

`click_bot_callback` принимает только recorded `chat_id/message_id/row/column`, находит
callback data в lossless reducer update и формирует `callbackQueryPayloadData` внутри core.
Answer `502` означает explicit `bot_timed_out`; local response deadline остаётся uncertain.
Оба исхода single-dispatch. Recorded outbound/reply IDs задают exact cleanup set; broad
delete/cleanup method не создаётся.

`open_web_app` возвращает scoped `WebAppLease`. Secret-bearing launch URL находится в
zeroizing/redacted `SensitiveString`; его нельзя логировать или помещать в result для
CLI/MCP. `wait_message_sent` принимает только matching `updateWebAppMessageSent` после
launch boundary. Explicit `close` и Drop fallback отправляют `closeWebApp`; Telegram
launch/message proof не является browser/DOM proof.

Для browser handoff lease можно disarm только после помещения URL в owner-scoped
in-memory artifact store daemon. One-shot handle живёт 60 секунд и забирается runner через
тот же private `0600` socket; expiry/take удаляют и zeroize URL. После browser evidence
caller закрывает exact launch отдельным `close_web_app_handoff` даже при failed UI.

`InputFileSource::Local/Generated` — только local core boundary. Daemon canonicalize-ит
source и configured `TDLIB_FILES_DIR`, требует absolute regular file внутри этого root и
отклоняет traversal/symlink escape. Remote MCP обязан использовать будущий owner-scoped
artifact handle и не может передавать произвольный server path; provider остаётся Q001/P9.
