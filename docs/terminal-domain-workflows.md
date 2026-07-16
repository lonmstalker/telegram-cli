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

`open_web_app` возвращает scoped `WebAppLease`. Secret-bearing launch URL находится в
zeroizing/redacted `SensitiveString`; его нельзя логировать или помещать в result для
CLI/MCP. `wait_message_sent` принимает только matching `updateWebAppMessageSent` после
launch boundary. Explicit `close` и Drop fallback отправляют `closeWebApp`; Telegram
launch/message proof не является browser/DOM proof.

`InputFileSource::Local/Generated` — только local core boundary. Daemon canonicalize-ит
source и configured `TDLIB_FILES_DIR`, требует absolute regular file внутри этого root и
отклоняет traversal/symlink escape. Remote MCP обязан использовать будущий owner-scoped
artifact handle и не может передавать произвольный server path; provider остаётся Q001/P9.
