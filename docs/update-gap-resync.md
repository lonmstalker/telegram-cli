# Update gap и resync

TDLib updates не содержат server sequence, а core receive loop не отбрасывает events.
Поэтому runtime не выводит gap из elapsed time или arbitrary queue threshold. Владелец
ordered stream вызывает `CoreRuntime::mark_update_gap` только при положительном evidence
потери/неприемлемого lag; detection thresholds и metrics принадлежат P5.

Marker запоминает последнюю применённую local sequence и остаётся установленным при
последующих updates. Пока он есть, state-dependent reads и mutations возвращают
structured `ResyncRequired` до dispatch. Pure raw calls и response-only pagination не
выдают cached-state proof и не очищают marker.

Единственный recovery path — policy-gated `resync_after_gap`:

1. вызывает `getCurrentState` и применяет/discards старые events до response boundary;
2. строит новый reducer из snapshot во временном value, продолжая monotonic local sequence;
3. заменяет старые caches и очищает marker только после успешной проверки всех updates;
4. оставляет старое gapped state без изменений, если snapshot некорректен.

Transient send/Web App/file caches, которых нет в snapshot, намеренно не переносятся.
После resync missing domain prerequisite требует обычной разрешённой hydration chain;
он не превращается в `not_found`. Raw `getCurrentState` response является server snapshot,
а не доказательством отсутствия будущих events.
