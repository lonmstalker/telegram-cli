# Per-account scheduler contract

Четвёртый P2 slice реализован в `telegramd::scheduler::AccountScheduler`. В MVP один scheduler принадлежит одному account profile; cross-account global queue не создаётся.

## Admission semantics

- Каждая operation получает monotonic FIFO ticket и class `Read` либо `Mutation`.
- Contiguous prefix reads может войти параллельно до переданного non-zero `max_concurrent_reads`.
- Mutation входит только с головы queue, когда нет active reads или другой mutation.
- Как только mutation стоит раньше read, поздний read её не обгоняет. После mutation следующий FIFO prefix снова может использовать read capacity.
- `OperationPermit` освобождает active slot через RAII. Drop ещё не admitted `QueuedOperation` отменяет ticket и будит следующих waiters.
- Ticket exhaustion fail closed; poison recovery сохраняет accounting state и не расширяет concurrency.

Read limit передаётся явным consumer configuration, а не фиксируется как Telegram truth. Этот пункт определяет admission mechanism; измеренный default/budgets и method-to-class data принадлежат P5. До generated registry P3 scheduler не угадывает class по имени метода.

## Current runtime boundary

Lease operations не проходят через scheduler: они управляют правом на session, а не являются TDLib calls. `AccountScheduler` подключается к TDLib dispatch после появления schema/capability data; текущий daemon всё ещё не отправляет рабочие requests в core.

## Verification

Threaded deterministic test удерживает два разрешённых read permits, ставит mutation перед late read и доказывает admission mutation первой после освобождения reads; late read входит только после mutation release. Отдельно проверены serialized second mutation и cancellation queued ticket.
