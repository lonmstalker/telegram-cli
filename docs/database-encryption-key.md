# Database encryption key contract

`telegram-core::database_key` закрывает protected-input boundary для `setTdlibParameters`. Ключ не является CLI/MCP argument и не читается из `.env.local` напрямую.

## Источники

- `FileDescriptor(OwnedFd)` принимает уже открытый brokered descriptor и consume-ит его без преобразования в model-visible integer argument.
- `FileSecret(PathBuf)` читает raw key bytes и требует absolute path, regular file текущего effective user, exact mode `0600`, `O_NOFOLLOW` и `O_CLOEXEC`.
- `Base64FileSecret(PathBuf)` применяет те же file gates, затем явно декодирует standard Base64 с допустимым одним завершающим LF/CRLF. Этот variant нужен для исторического `.env.local` file contract; auto-detect запрещён, чтобы текстовый raw key не интерпретировался неоднозначно.
- `OsKeychain` использует opaque service/account reference: macOS `security find-generic-password`, Linux Secret Service через `secret-tool`. Stdout захватывается в память и никогда не наследуется терминалом; command stderr не включается в ошибку.

Все источники отклоняют missing/empty key до TDLib; Base64 variant отдельно отклоняет malformed encoding. Чтение ограничено 4096 bytes на текущем secret-input boundary; raw/encoded buffers и временный Base64 encoder buffer zeroize-ятся при drop. Собранный request payload остаётся sensitive до немедленной передачи transport и не должен логироваться. `Debug` для key, source, TDLib parameters и authorization request не содержит secret value, path или keychain reference.

## TDJSON и fail-closed

Pinned `td_api.tl` объявляет `database_encryption_key:bytes`; pinned `ClientJson` декодирует JSON `bytes` из Base64. Поэтому raw key bytes кодируются standard padded Base64 только при построении `setTdlibParameters`.

Пустая строка не отправляется: pinned TDLib подменяет её internal default key, что несовместимо с C002. Ошибка TDLib `401` (`Wrong database encryption key`) переводит authorization machine в latched fail-closed state. Пока оператор явно не повторит parameters challenge с новым protected key, machine не принимает `WaitPhoneNumber` и не создаёт phone/QR request.

Core не выбирает default provider и не открывает TDLib DB: выбор reference принадлежит profile/deployment. Штатный `telegramd` теперь выбирает `TDLIB_DATABASE_KEY_FILE` как Base64 file secret только после canonical owner lock и один передаёт key в core; см. [session lifecycle contract](daemon-session-lifecycle.md).

Evidence для exact upstream codec/error semantics: [pinned source digest](../.memory/raw/2026-07-15-tdlib-database-key-codec.md).
