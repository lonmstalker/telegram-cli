//! Ограниченные retry для классов, где повтор доказанно безопасен.

use std::num::NonZeroUsize;
use std::thread;
use std::time::{Duration, Instant};

use crate::registry::{self, CapabilityDisposition, RetryClass};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttemptFailure<E> {
    Retryable { error: E, retry_after: Duration },
    Uncertain(E),
    Terminal(E),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DesiredState<T, E> {
    Reached(T),
    NotReached,
    Unknown(E),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryStop {
    Terminal,
    ReconciliationRequired,
    AttemptsExhausted,
    DeadlineExceeded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RetryExecution<T, E> {
    Succeeded {
        value: T,
        attempts: usize,
        reconciled: bool,
    },
    Stopped {
        error: E,
        attempts: usize,
        reason: RetryStop,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryPolicyError {
    UnknownMethod,
    DefaultDeny,
    WrongClass {
        expected: RetryClass,
        actual: RetryClass,
    },
}

pub fn safe_read<R, T, E>(
    method: &str,
    request: &R,
    max_attempts: NonZeroUsize,
    deadline: Instant,
    attempt: impl FnMut(&R) -> Result<T, AttemptFailure<E>>,
) -> Result<RetryExecution<T, E>, RetryPolicyError> {
    execute(
        method,
        RetryClass::SafeRead,
        request,
        max_attempts,
        deadline,
        attempt,
        |_| unreachable!("safe reads do not probe desired state"),
    )
}

pub fn convergent<R, T, E>(
    method: &str,
    request: &R,
    max_attempts: NonZeroUsize,
    deadline: Instant,
    attempt: impl FnMut(&R) -> Result<T, AttemptFailure<E>>,
    probe: impl FnMut(&R) -> DesiredState<T, E>,
) -> Result<RetryExecution<T, E>, RetryPolicyError> {
    execute(
        method,
        RetryClass::Convergent,
        request,
        max_attempts,
        deadline,
        attempt,
        probe,
    )
}

fn execute<R, T, E>(
    method: &str,
    class: RetryClass,
    request: &R,
    max_attempts: NonZeroUsize,
    deadline: Instant,
    mut attempt: impl FnMut(&R) -> Result<T, AttemptFailure<E>>,
    mut probe: impl FnMut(&R) -> DesiredState<T, E>,
) -> Result<RetryExecution<T, E>, RetryPolicyError> {
    require_class(method, class)?;
    let mut attempts = 0;
    loop {
        attempts += 1;
        match attempt(request) {
            Ok(value) => {
                return Ok(RetryExecution::Succeeded {
                    value,
                    attempts,
                    reconciled: false,
                });
            }
            Err(AttemptFailure::Terminal(error)) => {
                return Ok(stopped(error, attempts, RetryStop::Terminal));
            }
            Err(AttemptFailure::Uncertain(error)) => {
                return Ok(stopped(error, attempts, RetryStop::ReconciliationRequired));
            }
            Err(AttemptFailure::Retryable { error, retry_after }) => {
                if class == RetryClass::Convergent {
                    match probe(request) {
                        DesiredState::Reached(value) => {
                            return Ok(RetryExecution::Succeeded {
                                value,
                                attempts,
                                reconciled: true,
                            });
                        }
                        DesiredState::Unknown(error) => {
                            return Ok(stopped(error, attempts, RetryStop::ReconciliationRequired));
                        }
                        DesiredState::NotReached => {}
                    }
                }
                if attempts >= max_attempts.get() {
                    return Ok(stopped(error, attempts, RetryStop::AttemptsExhausted));
                }
                if !wait_until_retry(retry_after, deadline) {
                    return Ok(stopped(error, attempts, RetryStop::DeadlineExceeded));
                }
            }
        }
    }
}

fn require_class(method: &str, expected: RetryClass) -> Result<(), RetryPolicyError> {
    let capability = registry::capability(method).ok_or(RetryPolicyError::UnknownMethod)?;
    let CapabilityDisposition::Reviewed { retry: actual, .. } = capability.disposition else {
        return Err(RetryPolicyError::DefaultDeny);
    };
    if actual == expected {
        Ok(())
    } else {
        Err(RetryPolicyError::WrongClass { expected, actual })
    }
}

fn wait_until_retry(delay: Duration, deadline: Instant) -> bool {
    let now = Instant::now();
    let Some(not_before) = now.checked_add(delay) else {
        return false;
    };
    if not_before > deadline {
        return false;
    }
    thread::sleep(delay);
    Instant::now() <= deadline
}

fn stopped<T, E>(error: E, attempts: usize, reason: RetryStop) -> RetryExecution<T, E> {
    RetryExecution::Stopped {
        error,
        attempts,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_read_respects_server_delay_before_retry() {
        let request = 42;
        let started = Instant::now();
        let mut calls = 0;
        let result = safe_read(
            "getChat",
            &request,
            NonZeroUsize::new(2).unwrap(),
            started + Duration::from_secs(1),
            |actual| {
                assert!(std::ptr::eq(actual, &request));
                calls += 1;
                if calls == 1 {
                    Err(AttemptFailure::Retryable {
                        error: "flood",
                        retry_after: Duration::from_millis(20),
                    })
                } else {
                    Ok(*actual)
                }
            },
        )
        .unwrap();

        assert_eq!(
            result,
            RetryExecution::Succeeded {
                value: 42,
                attempts: 2,
                reconciled: false,
            }
        );
        assert!(started.elapsed() >= Duration::from_millis(20));
    }

    #[test]
    fn convergent_retries_same_request_only_after_probe() {
        let request = String::from("exact desired state");
        let mut calls = 0;
        let mut probes = 0;
        let result = convergent(
            "setChatDescription",
            &request,
            NonZeroUsize::new(2).unwrap(),
            Instant::now() + Duration::from_secs(1),
            |actual| {
                assert!(std::ptr::eq(actual, &request));
                calls += 1;
                if calls == 1 {
                    Err(AttemptFailure::Retryable {
                        error: "timeout",
                        retry_after: Duration::ZERO,
                    })
                } else {
                    Ok(actual.clone())
                }
            },
            |actual| {
                assert!(std::ptr::eq(actual, &request));
                probes += 1;
                DesiredState::NotReached
            },
        )
        .unwrap();

        assert_eq!(calls, 2);
        assert_eq!(probes, 1);
        assert_eq!(
            result,
            RetryExecution::Succeeded {
                value: request,
                attempts: 2,
                reconciled: false,
            }
        );
    }

    #[test]
    fn convergent_probe_can_prove_first_attempt_succeeded() {
        let result = convergent(
            "closeChat",
            &7,
            NonZeroUsize::new(2).unwrap(),
            Instant::now() + Duration::from_secs(1),
            |_| {
                Err(AttemptFailure::Retryable {
                    error: "timeout",
                    retry_after: Duration::from_secs(1),
                })
            },
            |_| DesiredState::Reached("already closed"),
        )
        .unwrap();

        assert_eq!(
            result,
            RetryExecution::Succeeded {
                value: "already closed",
                attempts: 1,
                reconciled: true,
            }
        );
    }

    #[test]
    fn reconcile_and_never_methods_cannot_enter_retry_executor() {
        for (method, actual) in [
            ("sendBotStartMessage", RetryClass::Reconcile),
            ("stopPoll", RetryClass::Never),
        ] {
            let result = safe_read(
                method,
                &(),
                NonZeroUsize::new(2).unwrap(),
                Instant::now() + Duration::from_secs(1),
                |_| Ok::<_, AttemptFailure<()>>(()),
            );
            assert_eq!(
                result,
                Err(RetryPolicyError::WrongClass {
                    expected: RetryClass::SafeRead,
                    actual,
                })
            );
        }
    }

    #[test]
    fn default_deny_method_cannot_dispatch() {
        let mut calls = 0;
        let result = safe_read(
            "testSquareInt",
            &(),
            NonZeroUsize::new(2).unwrap(),
            Instant::now() + Duration::from_secs(1),
            |_| {
                calls += 1;
                Ok::<_, AttemptFailure<()>>(())
            },
        );

        assert_eq!(result, Err(RetryPolicyError::DefaultDeny));
        assert_eq!(calls, 0);
    }

    #[test]
    fn uncertain_outcome_stops_without_retry() {
        let mut calls = 0;
        let result = convergent(
            "closeChat",
            &(),
            NonZeroUsize::new(3).unwrap(),
            Instant::now() + Duration::from_secs(1),
            |_| {
                calls += 1;
                Err::<(), _>(AttemptFailure::Uncertain("unknown outcome"))
            },
            |_| DesiredState::NotReached,
        )
        .unwrap();

        assert_eq!(calls, 1);
        assert_eq!(
            result,
            RetryExecution::Stopped {
                error: "unknown outcome",
                attempts: 1,
                reason: RetryStop::ReconciliationRequired,
            }
        );
    }
}
