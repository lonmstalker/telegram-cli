//! Fair admission одного account profile до TDLib dispatch.

use std::collections::VecDeque;
use std::fmt;
use std::num::NonZeroUsize;
use std::sync::{Arc, Condvar, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationClass {
    Read,
    Mutation,
}

#[derive(Clone)]
pub struct AccountScheduler {
    inner: Arc<SchedulerInner>,
}

struct SchedulerInner {
    max_concurrent_reads: usize,
    state: Mutex<SchedulerState>,
    changed: Condvar,
}

#[derive(Default)]
struct SchedulerState {
    next_ticket: u64,
    queue: VecDeque<Waiter>,
    active_reads: usize,
    mutation_active: bool,
}

#[derive(Clone, Copy)]
struct Waiter {
    ticket: u64,
    class: OperationClass,
}

impl AccountScheduler {
    pub fn new(max_concurrent_reads: NonZeroUsize) -> Self {
        Self {
            inner: Arc::new(SchedulerInner {
                max_concurrent_reads: max_concurrent_reads.get(),
                state: Mutex::new(SchedulerState {
                    next_ticket: 1,
                    ..SchedulerState::default()
                }),
                changed: Condvar::new(),
            }),
        }
    }

    pub fn enqueue(&self, class: OperationClass) -> Result<QueuedOperation, SchedulerError> {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let ticket = state.next_ticket;
        state.next_ticket = state
            .next_ticket
            .checked_add(1)
            .ok_or(SchedulerError::TicketExhausted)?;
        state.queue.push_back(Waiter { ticket, class });
        self.inner.changed.notify_all();
        Ok(QueuedOperation {
            inner: Arc::clone(&self.inner),
            ticket,
            class,
            queued: true,
        })
    }
}

pub struct QueuedOperation {
    inner: Arc<SchedulerInner>,
    ticket: u64,
    class: OperationClass,
    queued: bool,
}

impl QueuedOperation {
    pub fn wait(mut self) -> OperationPermit {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        while !can_admit(
            &state,
            self.ticket,
            self.class,
            self.inner.max_concurrent_reads,
        ) {
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
        match self.class {
            OperationClass::Read => state.active_reads += 1,
            OperationClass::Mutation => state.mutation_active = true,
        }
        self.queued = false;
        drop(state);
        self.inner.changed.notify_all();
        OperationPermit {
            inner: Arc::clone(&self.inner),
            class: self.class,
        }
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
                    .all(|waiter| waiter.class == OperationClass::Read)
        }
        OperationClass::Mutation => {
            position == 0 && !state.mutation_active && state.active_reads == 0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerError {
    TicketExhausted,
}

impl fmt::Display for SchedulerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TicketExhausted => formatter.write_str("scheduler ticket space is exhausted"),
        }
    }
}

impl std::error::Error for SchedulerError {}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::thread;

    use super::*;

    #[test]
    fn reads_are_bounded_and_late_read_cannot_overtake_mutation() {
        let scheduler = AccountScheduler::new(NonZeroUsize::new(2).unwrap());
        let first_read = scheduler.enqueue(OperationClass::Read).unwrap().wait();
        let second_read = scheduler.enqueue(OperationClass::Read).unwrap().wait();
        let mutation = scheduler.enqueue(OperationClass::Mutation).unwrap();
        let late_read = scheduler.enqueue(OperationClass::Read).unwrap();
        assert_eq!(scheduler.inner.state.lock().unwrap().active_reads, 2);

        let (admitted_tx, admitted_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let mutation_thread = thread::spawn({
            let admitted_tx = admitted_tx.clone();
            move || {
                let permit = mutation.wait();
                admitted_tx.send(OperationClass::Mutation).unwrap();
                release_rx.recv().unwrap();
                drop(permit);
            }
        });
        let read_thread = thread::spawn(move || {
            let _permit = late_read.wait();
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
    fn dropping_queued_operation_cancels_its_ticket() {
        let scheduler = AccountScheduler::new(NonZeroUsize::new(1).unwrap());
        let mutation = scheduler.enqueue(OperationClass::Mutation).unwrap().wait();
        let waiting_mutation = scheduler.enqueue(OperationClass::Mutation).unwrap();
        let cancelled = scheduler.enqueue(OperationClass::Read).unwrap();
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
        drop(scheduler.enqueue(OperationClass::Read).unwrap().wait());
    }
}
