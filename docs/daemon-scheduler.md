# Per-account scheduler contract

Четвёртый P2 slice реализован в `telegramd::scheduler::AccountScheduler`. В MVP один scheduler принадлежит одному account profile; cross-account global queue не создаётся.

## Admission semantics

- Каждая operation получает monotonic FIFO ticket. `enqueue_method` берёт risk class из
  generated capability data; только `read` получает concurrency class `Read`, остальные
  reviewed risks консервативно сериализуются как `Mutation`. Default-deny method не входит
  в queue.
- Contiguous prefix reads может войти параллельно до переданного non-zero `max_concurrent_reads`.
- Mutation входит только с головы queue, когда нет active reads или другой mutation.
- Как только mutation стоит раньше read, поздний read её не обгоняет. После mutation следующий FIFO prefix снова может использовать read capacity.
- `OperationPermit` освобождает active slot через RAII. Drop ещё не admitted `QueuedOperation` отменяет ticket и будит следующих waiters.
- Ticket exhaustion fail closed; poison recovery сохраняет accounting state и не расширяет concurrency.

## Queue и rate budgets

`SchedulerBudgets` передаётся consumer явно и не имеет guessed defaults. Для account,
каждого chat и каждого generated `RiskClass` задаётся один `ScopeBudget`:

- `max_queued` ограничивает ожидающие tickets до их выдачи;
- `RateBudget` задаёт non-zero maximum/window для dispatch timestamps.

Отсутствующий budget хотя бы для одного risk class запрещает создание scheduler. Operation
без chat ID использует account и method-class budgets; Telegram identifiers не попадают в
error text. Один bounded timestamp deque обслуживает все три измерения, второй queue/rate
framework не создаётся.

## Flood delay и jitter

`record_flood_wait` принимает explicit account/chat/method-class scope и server delay.
Scope остаётся blocked не меньше server delay. Если delay помещается в configured
`max_automatic_backoff`, к нему добавляется bounded jitter не выше `max_jitter` и общего
maximum. Если server delay больше automatic budget, scheduler сохраняет весь server block,
но возвращает `automatic_delay=None`: caller не может автоматически retry раньше или
обрезать Telegram delay.

Read/queue/rate/backoff limits не фиксируются как Telegram truth. Текущий однопоточный
socket consumer использует technical serial budget и effectively unbounded rate window;
полученный от TDLib flood delay блокирует generated method class и передаётся bounded
safe-read retry без сокращения.

## Current runtime boundary

Lease operations не проходят через scheduler: они управляют правом на session, а не являются
TDLib calls. Universal raw dispatch входит в scheduler после policy/approval и до journal/
transport. Curated workflows уже сериализованы одним daemon request loop; их внутренние
TDLib reads используют тот же generated safe-read retry, а mutation reconciliation остаётся
в typed workflow. Измеренные multi-read production budgets остаются Q001.

## Verification

Tests удерживают два read permits и доказывают FIFO mutation barrier; отдельно проверяют
независимые account/chat/method rate windows, fail-closed queue dimensions, generated method
classification/default-deny, bounded flood jitter и отсутствие automatic retry при delay
выше configured bound.
