//! Fair admission и explicit rate/flood budgets одного account profile.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime};

use telegram_core::registry::{self, CapabilityDisposition, RiskClass};

use crate::telemetry::Telemetry;

const RISK_CLASSES: [RiskClass; 8] = [
    RiskClass::Read,
    RiskClass::Presence,
    RiskClass::Send,
    RiskClass::ReversibleMutation,
    RiskClass::Admin,
    RiskClass::Destructive,
    RiskClass::Financial,
    RiskClass::AuthSecurity,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationClass {
    Read,
    Mutation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationContext {
    pub operation: OperationClass,
    pub method_class: RiskClass,
    pub chat_id: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateBudget {
    maximum: NonZeroU32,
    window: Duration,
}

impl RateBudget {
    pub fn new(maximum: NonZeroU32, window: Duration) -> Result<Self, SchedulerError> {
        if window.is_zero() {
            return Err(SchedulerError::ZeroRateWindow);
        }
        Ok(Self { maximum, window })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopeBudget {
    pub max_queued: NonZeroUsize,
    pub rate: RateBudget,
}

pub struct SchedulerBudgets {
    pub max_concurrent_reads: NonZeroUsize,
    pub account: ScopeBudget,
    pub chat: ScopeBudget,
    pub method_classes: BTreeMap<RiskClass, ScopeBudget>,
    pub max_automatic_backoff: Duration,
    pub max_jitter: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloodScope {
    Account,
    Chat(i64),
    MethodClass(RiskClass),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackoffDecision {
    pub server_delay: Duration,
    pub automatic_delay: Option<Duration>,
}

#[derive(Clone)]
pub struct AccountScheduler {
    inner: Arc<SchedulerInner>,
}

struct SchedulerInner {
    budgets: SchedulerBudgets,
    max_rate_window: Duration,
    state: Mutex<SchedulerState>,
    changed: Condvar,
    jitter_counter: AtomicU64,
    telemetry: Telemetry,
}

#[derive(Default)]
struct SchedulerState {
    next_ticket: u64,
    queue: VecDeque<Waiter>,
    active_reads: usize,
    mutation_active: bool,
    dispatches: VecDeque<DispatchStamp>,
    account_blocked_until: Option<Instant>,
    chat_blocked_until: BTreeMap<i64, Instant>,
    method_blocked_until: BTreeMap<RiskClass, Instant>,
}

#[derive(Clone, Copy)]
struct Waiter {
    ticket: u64,
    context: OperationContext,
}

#[derive(Clone, Copy)]
struct DispatchStamp {
    at: Instant,
    chat_id: Option<i64>,
    method_class: RiskClass,
}

impl AccountScheduler {
    pub fn new(budgets: SchedulerBudgets) -> Result<Self, SchedulerError> {
        Self::with_telemetry(budgets, Telemetry::default())
    }

    pub fn with_telemetry(
        budgets: SchedulerBudgets,
        telemetry: Telemetry,
    ) -> Result<Self, SchedulerError> {
        if RISK_CLASSES
            .iter()
            .any(|class| !budgets.method_classes.contains_key(class))
        {
            return Err(SchedulerError::MissingMethodClassBudget);
        }
        let max_rate_window = budgets
            .method_classes
            .values()
            .map(|budget| budget.rate.window)
            .chain([budgets.account.rate.window, budgets.chat.rate.window])
            .max()
            .expect("account and chat budgets are present");
        Ok(Self {
            inner: Arc::new(SchedulerInner {
                budgets,
                max_rate_window,
                state: Mutex::new(SchedulerState {
                    next_ticket: 1,
                    ..SchedulerState::default()
                }),
                changed: Condvar::new(),
                jitter_counter: AtomicU64::new(1),
                telemetry,
            }),
        })
    }

    pub fn enqueue(&self, context: OperationContext) -> Result<QueuedOperation, SchedulerError> {
        let mut state = self.lock_state();
        let method_budget = self
            .inner
            .budgets
            .method_classes
            .get(&context.method_class)
            .ok_or(SchedulerError::MissingMethodClassBudget)?;
        if state.queue.len() >= self.inner.budgets.account.max_queued.get() {
            self.inner.telemetry.record_queue_rejection();
            return Err(SchedulerError::QueueBudgetExceeded(QueueDimension::Account));
        }
        if context.chat_id.is_some_and(|chat_id| {
            state
                .queue
                .iter()
                .filter(|waiter| waiter.context.chat_id == Some(chat_id))
                .count()
                >= self.inner.budgets.chat.max_queued.get()
        }) {
            self.inner.telemetry.record_queue_rejection();
            return Err(SchedulerError::QueueBudgetExceeded(QueueDimension::Chat));
        }
        if state
            .queue
            .iter()
            .filter(|waiter| waiter.context.method_class == context.method_class)
            .count()
            >= method_budget.max_queued.get()
        {
            self.inner.telemetry.record_queue_rejection();
            return Err(SchedulerError::QueueBudgetExceeded(
                QueueDimension::MethodClass,
            ));
        }
        let ticket = state.next_ticket;
        state.next_ticket = state
            .next_ticket
            .checked_add(1)
            .ok_or(SchedulerError::TicketExhausted)?;
        state.queue.push_back(Waiter { ticket, context });
        self.inner.telemetry.observe_queue(state.queue.len());
        self.inner.changed.notify_all();
        Ok(QueuedOperation {
            inner: Arc::clone(&self.inner),
            ticket,
            context,
            queued: true,
        })
    }

    pub fn enqueue_method(
        &self,
        method: &str,
        chat_id: Option<i64>,
    ) -> Result<QueuedOperation, SchedulerError> {
        let capability = registry::capability(method).ok_or(SchedulerError::MethodNotReviewed)?;
        let CapabilityDisposition::Reviewed { risk, .. } = capability.disposition else {
            return Err(SchedulerError::MethodNotReviewed);
        };
        self.enqueue(OperationContext {
            operation: if risk == RiskClass::Read {
                OperationClass::Read
            } else {
                OperationClass::Mutation
            },
            method_class: risk,
            chat_id,
        })
    }

    pub fn record_flood_wait(
        &self,
        scope: FloodScope,
        server_delay: Duration,
    ) -> Result<BackoffDecision, SchedulerError> {
        self.inner.telemetry.record_flood(server_delay);
        let automatic_delay = self.automatic_delay(server_delay);
        let blocked_for = automatic_delay.unwrap_or(server_delay);
        let blocked_until = Instant::now()
            .checked_add(blocked_for)
            .ok_or(SchedulerError::DeadlineOverflow)?;
        let mut state = self.lock_state();
        match scope {
            FloodScope::Account => extend_block(&mut state.account_blocked_until, blocked_until),
            FloodScope::Chat(chat_id) => {
                extend_map_block(&mut state.chat_blocked_until, chat_id, blocked_until)
            }
            FloodScope::MethodClass(class) => {
                extend_map_block(&mut state.method_blocked_until, class, blocked_until)
            }
        }
        drop(state);
        self.inner.changed.notify_all();
        Ok(BackoffDecision {
            server_delay,
            automatic_delay,
        })
    }

    fn automatic_delay(&self, server_delay: Duration) -> Option<Duration> {
        let maximum = self.inner.budgets.max_automatic_backoff;
        let headroom = maximum.checked_sub(server_delay)?;
        let jitter_cap = self.inner.budgets.max_jitter.min(headroom);
        Some(server_delay + self.jitter(jitter_cap))
    }

    fn jitter(&self, maximum: Duration) -> Duration {
        if maximum.is_zero() {
            return Duration::ZERO;
        }
        let cap = maximum.as_nanos().min(u128::from(u64::MAX)) as u64;
        let counter = self.inner.jitter_counter.fetch_add(1, Ordering::Relaxed);
        let clock = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let sample = counter.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ u64::from(clock);
        Duration::from_nanos(sample % cap.saturating_add(1))
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, SchedulerState> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub struct QueuedOperation {
    inner: Arc<SchedulerInner>,
    ticket: u64,
    context: OperationContext,
    queued: bool,
}

impl QueuedOperation {
    pub fn wait(mut self) -> Result<OperationPermit, SchedulerError> {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        loop {
            let now = Instant::now();
            prune(&mut state, now, self.inner.max_rate_window);
            if can_admit(
                &state,
                self.ticket,
                self.context.operation,
                self.inner.budgets.max_concurrent_reads.get(),
            ) {
                if let Some(ready_at) =
                    admission_ready_at(&state, self.context, &self.inner.budgets, now)?
                {
                    let timeout = ready_at.saturating_duration_since(now);
                    let (next, _) = self
                        .inner
                        .changed
                        .wait_timeout(state, timeout)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    state = next;
                    continue;
                }
                break;
            }
            state = self
                .inner
                .changed
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }

        let position = state
            .queue
            .iter()
            .position(|waiter| waiter.ticket == self.ticket)
            .expect("queued scheduler ticket disappeared");
        state.queue.remove(position);
        self.inner.telemetry.observe_queue(state.queue.len());
        match self.context.operation {
            OperationClass::Read => state.active_reads += 1,
            OperationClass::Mutation => state.mutation_active = true,
        }
        state.dispatches.push_back(DispatchStamp {
            at: Instant::now(),
            chat_id: self.context.chat_id,
            method_class: self.context.method_class,
        });
        self.queued = false;
        drop(state);
        self.inner.changed.notify_all();
        Ok(OperationPermit {
            inner: Arc::clone(&self.inner),
            class: self.context.operation,
        })
    }
}

impl Drop for QueuedOperation {
    fn drop(&mut self) {
        if !self.queued {
            return;
        }
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(position) = state
            .queue
            .iter()
            .position(|waiter| waiter.ticket == self.ticket)
        {
            state.queue.remove(position);
            self.inner.telemetry.observe_queue(state.queue.len());
            self.inner.changed.notify_all();
        }
    }
}

pub struct OperationPermit {
    inner: Arc<SchedulerInner>,
    class: OperationClass,
}

impl Drop for OperationPermit {
    fn drop(&mut self) {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match self.class {
            OperationClass::Read => {
                state.active_reads = state
                    .active_reads
                    .checked_sub(1)
                    .expect("read permit accounting underflow");
            }
            OperationClass::Mutation => state.mutation_active = false,
        }
        drop(state);
        self.inner.changed.notify_all();
    }
}

fn admission_ready_at(
    state: &SchedulerState,
    context: OperationContext,
    budgets: &SchedulerBudgets,
    now: Instant,
) -> Result<Option<Instant>, SchedulerError> {
    let method_budget = budgets
        .method_classes
        .get(&context.method_class)
        .ok_or(SchedulerError::MissingMethodClassBudget)?;
    let rate_ready = [
        rate_ready_at(&state.dispatches, budgets.account.rate, now, |_| true)?,
        context.chat_id.map_or(Ok(None), |chat_id| {
            rate_ready_at(&state.dispatches, budgets.chat.rate, now, |stamp| {
                stamp.chat_id == Some(chat_id)
            })
        })?,
        rate_ready_at(&state.dispatches, method_budget.rate, now, |stamp| {
            stamp.method_class == context.method_class
        })?,
    ]
    .into_iter()
    .flatten()
    .max();
    let flood_ready = [
        state.account_blocked_until,
        context
            .chat_id
            .and_then(|chat_id| state.chat_blocked_until.get(&chat_id).copied()),
        state
            .method_blocked_until
            .get(&context.method_class)
            .copied(),
    ]
    .into_iter()
    .flatten()
    .filter(|until| *until > now)
    .max();
    Ok(rate_ready.into_iter().chain(flood_ready).max())
}

fn rate_ready_at(
    dispatches: &VecDeque<DispatchStamp>,
    budget: RateBudget,
    now: Instant,
    matches: impl Fn(&DispatchStamp) -> bool,
) -> Result<Option<Instant>, SchedulerError> {
    let maximum = budget.maximum.get() as usize;
    let relevant = dispatches
        .iter()
        .filter(|stamp| matches(stamp))
        .collect::<Vec<_>>();
    if relevant.len() < maximum {
        return Ok(None);
    }
    let ready_at = relevant[relevant.len() - maximum]
        .at
        .checked_add(budget.window)
        .ok_or(SchedulerError::DeadlineOverflow)?;
    Ok((ready_at > now).then_some(ready_at))
}

fn prune(state: &mut SchedulerState, now: Instant, max_window: Duration) {
    while state
        .dispatches
        .front()
        .is_some_and(|stamp| now.saturating_duration_since(stamp.at) >= max_window)
    {
        state.dispatches.pop_front();
    }
    if state
        .account_blocked_until
        .is_some_and(|until| until <= now)
    {
        state.account_blocked_until = None;
    }
    state.chat_blocked_until.retain(|_, until| *until > now);
    state.method_blocked_until.retain(|_, until| *until > now);
}

fn extend_block(current: &mut Option<Instant>, candidate: Instant) {
    if current.is_none_or(|known| candidate > known) {
        *current = Some(candidate);
    }
}

fn extend_map_block<Key: Ord + Copy>(
    blocks: &mut BTreeMap<Key, Instant>,
    key: Key,
    candidate: Instant,
) {
    blocks
        .entry(key)
        .and_modify(|known| *known = (*known).max(candidate))
        .or_insert(candidate);
}

fn can_admit(
    state: &SchedulerState,
    ticket: u64,
    class: OperationClass,
    max_concurrent_reads: usize,
) -> bool {
    let Some(position) = state
        .queue
        .iter()
        .position(|waiter| waiter.ticket == ticket)
    else {
        return false;
    };
    match class {
        OperationClass::Read => {
            !state.mutation_active
                && state.active_reads < max_concurrent_reads
                && state
                    .queue
                    .iter()
                    .take(position)
                    .all(|waiter| waiter.context.operation == OperationClass::Read)
        }
        OperationClass::Mutation => {
            position == 0 && !state.mutation_active && state.active_reads == 0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueDimension {
    Account,
    Chat,
    MethodClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerError {
    TicketExhausted,
    ZeroRateWindow,
    MissingMethodClassBudget,
    MethodNotReviewed,
    QueueBudgetExceeded(QueueDimension),
    DeadlineOverflow,
}

impl fmt::Display for SchedulerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TicketExhausted => formatter.write_str("scheduler ticket space is exhausted"),
            Self::ZeroRateWindow => formatter.write_str("rate budget window must be non-zero"),
            Self::MissingMethodClassBudget => {
                formatter.write_str("every generated risk class requires an explicit budget")
            }
            Self::MethodNotReviewed => {
                formatter.write_str("TDLib method has no reviewed scheduler class")
            }
            Self::QueueBudgetExceeded(dimension) => {
                write!(
                    formatter,
                    "{dimension:?} scheduler queue budget is exhausted"
                )
            }
            Self::DeadlineOverflow => formatter.write_str("scheduler deadline overflow"),
        }
    }
}

impl std::error::Error for SchedulerError {}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::thread;

    use super::*;

    fn rate(maximum: u32, milliseconds: u64) -> RateBudget {
        RateBudget::new(
            NonZeroU32::new(maximum).unwrap(),
            Duration::from_millis(milliseconds),
        )
        .unwrap()
    }

    fn budgets(max_reads: usize) -> SchedulerBudgets {
        let scope = ScopeBudget {
            max_queued: NonZeroUsize::new(100).unwrap(),
            rate: rate(100, 1_000),
        };
        SchedulerBudgets {
            max_concurrent_reads: NonZeroUsize::new(max_reads).unwrap(),
            account: scope,
            chat: scope,
            method_classes: RISK_CLASSES
                .into_iter()
                .map(|class| (class, scope))
                .collect(),
            max_automatic_backoff: Duration::from_secs(10),
            max_jitter: Duration::from_secs(1),
        }
    }

    fn operation(class: OperationClass, risk: RiskClass, chat_id: i64) -> OperationContext {
        OperationContext {
            operation: class,
            method_class: risk,
            chat_id: Some(chat_id),
        }
    }

    #[test]
    fn reads_are_bounded_and_late_read_cannot_overtake_mutation() {
        let scheduler = AccountScheduler::new(budgets(2)).unwrap();
        let read = operation(OperationClass::Read, RiskClass::Read, 1);
        let mutation_context = operation(OperationClass::Mutation, RiskClass::Send, 1);
        let first_read = scheduler.enqueue(read).unwrap().wait().unwrap();
        let second_read = scheduler.enqueue(read).unwrap().wait().unwrap();
        let mutation = scheduler.enqueue(mutation_context).unwrap();
        let late_read = scheduler.enqueue(read).unwrap();
        assert_eq!(scheduler.inner.state.lock().unwrap().active_reads, 2);

        let (admitted_tx, admitted_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let mutation_thread = thread::spawn({
            let admitted_tx = admitted_tx.clone();
            move || {
                let permit = mutation.wait().unwrap();
                admitted_tx.send(OperationClass::Mutation).unwrap();
                release_rx.recv().unwrap();
                drop(permit);
            }
        });
        let read_thread = thread::spawn(move || {
            let _permit = late_read.wait().unwrap();
            admitted_tx.send(OperationClass::Read).unwrap();
        });

        drop(first_read);
        drop(second_read);
        assert_eq!(admitted_rx.recv().unwrap(), OperationClass::Mutation);
        assert!(admitted_rx.try_recv().is_err());
        release_tx.send(()).unwrap();
        assert_eq!(admitted_rx.recv().unwrap(), OperationClass::Read);
        mutation_thread.join().unwrap();
        read_thread.join().unwrap();
    }

    #[test]
    fn method_class_comes_from_generated_capability_data() {
        let scheduler = AccountScheduler::new(budgets(1)).unwrap();
        let read = scheduler.enqueue_method("getChat", Some(1)).unwrap();
        assert_eq!(read.context.operation, OperationClass::Read);
        assert_eq!(read.context.method_class, RiskClass::Read);
        drop(read);

        let send = scheduler
            .enqueue_method("sendBotStartMessage", Some(1))
            .unwrap();
        assert_eq!(send.context.operation, OperationClass::Mutation);
        assert_eq!(send.context.method_class, RiskClass::Send);
        drop(send);
        assert!(matches!(
            scheduler.enqueue_method("testSquareInt", None),
            Err(SchedulerError::MethodNotReviewed)
        ));
    }

    #[test]
    fn account_chat_and_method_rate_budgets_are_independent() {
        let mut configured = budgets(4);
        configured.account.rate = rate(3, 100);
        configured.chat.rate = rate(1, 100);
        configured
            .method_classes
            .get_mut(&RiskClass::Read)
            .unwrap()
            .rate = rate(2, 100);
        let scheduler = AccountScheduler::new(configured).unwrap();
        let chat_one = operation(OperationClass::Read, RiskClass::Read, 1);
        let chat_two = operation(OperationClass::Read, RiskClass::Read, 2);
        drop(scheduler.enqueue(chat_one).unwrap().wait().unwrap());
        drop(scheduler.enqueue(chat_two).unwrap().wait().unwrap());

        let state = scheduler.inner.state.lock().unwrap();
        let now = Instant::now();
        assert!(
            rate_ready_at(
                &state.dispatches,
                scheduler.inner.budgets.account.rate,
                now,
                |_| true
            )
            .unwrap()
            .is_none()
        );
        assert!(
            rate_ready_at(
                &state.dispatches,
                scheduler.inner.budgets.chat.rate,
                now,
                |stamp| stamp.chat_id == Some(1)
            )
            .unwrap()
            .is_some()
        );
        assert!(
            rate_ready_at(
                &state.dispatches,
                scheduler.inner.budgets.method_classes[&RiskClass::Read].rate,
                now,
                |stamp| stamp.method_class == RiskClass::Read
            )
            .unwrap()
            .is_some()
        );
    }

    #[test]
    fn flood_backoff_never_precedes_server_delay_and_is_bounded() {
        let scheduler = AccountScheduler::new(budgets(1)).unwrap();
        let decision = scheduler
            .record_flood_wait(
                FloodScope::MethodClass(RiskClass::Read),
                Duration::from_secs(3),
            )
            .unwrap();
        assert!(decision.automatic_delay.unwrap() >= decision.server_delay);
        assert!(decision.automatic_delay.unwrap() <= Duration::from_secs(4));
        assert!(
            scheduler.inner.state.lock().unwrap().method_blocked_until[&RiskClass::Read]
                > Instant::now()
        );

        let too_long = scheduler
            .record_flood_wait(FloodScope::Account, Duration::from_secs(11))
            .unwrap();
        assert_eq!(too_long.automatic_delay, None);
        assert_eq!(too_long.server_delay, Duration::from_secs(11));
    }

    #[test]
    fn queue_budgets_fail_closed_by_dimension() {
        let mut configured = budgets(1);
        configured.chat.max_queued = NonZeroUsize::new(1).unwrap();
        configured
            .method_classes
            .get_mut(&RiskClass::Send)
            .unwrap()
            .max_queued = NonZeroUsize::new(1).unwrap();
        let scheduler = AccountScheduler::new(configured).unwrap();
        let active = scheduler
            .enqueue(operation(OperationClass::Mutation, RiskClass::Admin, 0))
            .unwrap()
            .wait()
            .unwrap();
        let waiting = scheduler
            .enqueue(operation(OperationClass::Mutation, RiskClass::Send, 1))
            .unwrap();
        assert!(matches!(
            scheduler.enqueue(operation(OperationClass::Mutation, RiskClass::Send, 1)),
            Err(SchedulerError::QueueBudgetExceeded(QueueDimension::Chat))
        ));
        assert!(matches!(
            scheduler.enqueue(operation(OperationClass::Mutation, RiskClass::Send, 2)),
            Err(SchedulerError::QueueBudgetExceeded(
                QueueDimension::MethodClass
            ))
        ));
        drop(waiting);
        drop(active);

        let mut configured = budgets(1);
        configured.account.max_queued = NonZeroUsize::new(1).unwrap();
        let scheduler = AccountScheduler::new(configured).unwrap();
        let active = scheduler
            .enqueue(operation(OperationClass::Mutation, RiskClass::Admin, 0))
            .unwrap()
            .wait()
            .unwrap();
        let waiting = scheduler
            .enqueue(operation(OperationClass::Mutation, RiskClass::Send, 1))
            .unwrap();
        assert!(matches!(
            scheduler.enqueue(operation(OperationClass::Read, RiskClass::Read, 2)),
            Err(SchedulerError::QueueBudgetExceeded(QueueDimension::Account))
        ));
        drop(waiting);
        drop(active);
    }

    #[test]
    fn dropping_queued_operation_cancels_its_ticket() {
        let scheduler = AccountScheduler::new(budgets(1)).unwrap();
        let mutation_context = operation(OperationClass::Mutation, RiskClass::Send, 1);
        let read = operation(OperationClass::Read, RiskClass::Read, 1);
        let mutation = scheduler.enqueue(mutation_context).unwrap().wait().unwrap();
        let waiting_mutation = scheduler.enqueue(mutation_context).unwrap();
        let cancelled = scheduler.enqueue(read).unwrap();
        assert!(!can_admit(
            &scheduler.inner.state.lock().unwrap(),
            waiting_mutation.ticket,
            OperationClass::Mutation,
            1
        ));
        drop(cancelled);
        drop(waiting_mutation);
        assert!(scheduler.inner.state.lock().unwrap().queue.is_empty());
        drop(mutation);
        drop(scheduler.enqueue(read).unwrap().wait().unwrap());
    }
}
