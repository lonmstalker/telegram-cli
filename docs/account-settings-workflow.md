# Notification settings и active sessions

F016 оставляет password/recovery/OTP/device registration в generated raw/default-deny
surface, пока для secret-bearing inputs нет отдельного protected consumer. Curated route
закрывают routine desired state и exact remote-session termination:

```text
notification_settings        set_notification_settings
active_sessions
plan_terminate_session       apply_terminate_session
```

Notification scope — closed enum `private_chats|group_chats|channel_chats`. Setter принимает
partial `patch`: missing поля сначала читаются с сервера и копируются без изменения. Пустой
или уже достигнутый patch не dispatch-ится. После success либо timeout весь scope читается
заново; только полное совпадение merged desired state даёт `verified`.

`active_sessions` возвращает exact session ID и только безопасные targeting/status fields:
`is_current`, password/unconfirmed flags, official-app flag и last-active timestamp.
IP, location, device model, platform, system/application version не входят в result; output
имеет explicit `sensitive_metadata_redacted=true`.

Termination использует только exact `session_id`. Plan classified `auth_security` и требует
external one-shot approval. Apply всегда получает fresh session list, отклоняет current
session и не имеет broad `terminate all` action. Success и response timeout завершаются
вторым `getActiveSessions`; отсутствие exact ID доказывает completion, наличие/failed read
даёт `uncertain` без blind retry.

Synthetic test проверяет сохранение omitted notification fields, отсутствие private session
canaries, запрет current session и verified refresh после потерянного terminate response.
Live setting/session mutation не выполнялась и относится к P10 с отдельным разрешением
владельца перед session side effect.
