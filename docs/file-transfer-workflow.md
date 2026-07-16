# F010 file/media workflows

`download_file` сохраняет async `downloadFile` semantics: response с file ID или progress
не terminal; workflow ждёт matching ordered `updateFile.local.is_downloading_completed`.
Offset/limit передаются как bounded resume slice. `upload_sticker_file` использует тот же
engine для `remote.is_uploading_completed` и фактического upload prerequisite.

Terminal receipt проверяет `file.size`, а при неизвестном actual size — `expected_size`,
против `downloaded_size|uploaded_size`. Известный mismatch не становится complete;
`size_verified=false` допустим только когда TDLib не сообщил размер. Checksum в pinned
File object отсутствует и не выдумывается.

`cancel_download` — desired-state workflow: cached/server `getFile` уже inactive не
dispatch-ится; после `cancelDownloadFile`, включая response timeout, повторный `getFile`
обязан показать inactive. Иначе receipt `uncertain/complete=false`, без blind cancel loop.

Local/generated paths проходят только на daemon. Production root — configured
`TDLIB_FILES_DIR`; обе стороны canonicalized, source обязан быть absolute regular file
внутри root. Outside path, missing root и symlink escape дают `InvalidWorkflowInput` до
TDLib dispatch. ID/remote sources не интерпретируются как filesystem paths.

Remote MCP artifact provider остаётся explicit Q001 и не подменяется произвольным server
path. Остальные file/import/autosave/generated methods доступны через universal generated
raw registry и остаются default-deny до фактического capability consumer.

Behavior tests покрывают terminal update+size, cancel reconciliation, gap blocking и
path/symlink confinement. Live transfer не выполнялся без disposable artifact; P10 boundary.
