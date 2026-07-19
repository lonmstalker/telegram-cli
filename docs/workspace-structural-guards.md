# Workspace structural guards

`python3 scripts/check-workspace-boundaries.py` остаётся общим harness-gate для Cargo workspace. После проверки package topology и четырёх встроенных negative controls он последовательно запускает три отдельных guard-скрипта. Активных CI workflow и git-хуков в репозитории нет, поэтому canonical точка запуска — та же harness-процедура, что использовалась для workspace boundary раньше.

## Размер Rust-файлов

`scripts/check-source-file-size.py` проверяет все `*.rs` в `crates/` и `apps/`. Новый или обычный файл не может превышать 1500 строк; `*generated.rs` исключены как generated output.

Текущие legacy-нарушители закреплены ratchet-списком в самом скрипте:

- `apps/telegramd/src/server.rs` — максимум 2542 строк;
- `crates/telegram-core/src/workflows/mod.rs` — максимум 2146 строк.

Ratchet-файл должен совпадать с зафиксированным размером: рост запрещён, а уменьшение выше порога требует снизить записанный размер в том же change. Когда файл уменьшается до 1500 строк или меньше, gate требует удалить устаревшую запись из `RATCHET`; увеличение лимита не является штатным способом исправления.

## Единственный daemon socket client

`scripts/check-daemon-client-single-home.py` grep-level scan запрещает в consumer-коде `apps/*` определения `fn socket_path`, `fn validate_socket` и прямой `UnixStream::connect`. Единственный client home — `crates/telegram-client`.

`apps/telegramd` не является consumer и исключён из scan: сервер владеет `UnixListener`, socket pathname и stale-socket connect probes для election/recovery. Это исключение не распространяется на остальные приложения.

## Workspace dependencies

`scripts/check-workspace-dependencies.py` читает корневые `[workspace].members` и `[workspace.dependencies]`, затем проверяет обычные, dev-, build- и target-specific dependency tables каждого member. Зависимость, присутствующая в `[workspace.dependencies]`, должна наследоваться через `workspace = true`; локальные `features = [...]` поверх наследования разрешены. Явные `version`, `path`, `git` или `registry` в member считаются обходом общего контракта.

Зависимости, которых нет в `[workspace.dependencies]`, остаются локальными и этим guard не ограничиваются.

## Проверка самих guards

Каждый скрипт имеет paired fixture-test с positive и negative controls на временном мини-репозитории:

```console
python3 scripts/test-source-file-size.py
python3 scripts/test-daemon-client-single-home.py
python3 scripts/test-workspace-dependencies.py
```

Прямой запуск отдельных guards доступен для локальной диагностики, но полный structural contract запускается одной командой:

```console
python3 scripts/check-workspace-boundaries.py
```
