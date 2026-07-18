use std::collections::VecDeque;

use telegram_protocol::{
    ClientErrorCode, DaemonRequest, DaemonResponse, LoginChallengeId, LoginInput, LoginNextAction,
    LoginState, OwnerLoginPrompt, ProtectedString,
};

use super::{LoginAction, LoginBroker, LoginDriver, LoginRuntime, OwnerPrompt};
use crate::CliError;

struct FakeBroker {
    script: VecDeque<(DaemonRequest, DaemonResponse)>,
}

impl LoginBroker for FakeBroker {
    fn exchange(&mut self, request: DaemonRequest) -> Result<DaemonResponse, CliError> {
        let (expected, response) = self.script.pop_front().expect("unexpected broker call");
        assert_eq!(request, expected);
        Ok(response)
    }
}

struct FakePrompt {
    actions: VecDeque<LoginAction>,
    notices: Vec<&'static str>,
}

impl OwnerPrompt for FakePrompt {
    fn action(
        &mut self,
        _state: LoginState,
        _prompt: OwnerLoginPrompt,
    ) -> Result<LoginAction, CliError> {
        Ok(self.actions.pop_front().expect("unexpected owner prompt"))
    }

    fn notice(&mut self, message: &'static str) -> Result<(), CliError> {
        self.notices.push(message);
        Ok(())
    }
}

#[derive(Default)]
struct FakeRuntime {
    cancelled: bool,
    waits: usize,
}

impl LoginRuntime for FakeRuntime {
    fn cancelled(&self) -> bool {
        self.cancelled
    }

    fn wait(&mut self) {
        self.waits += 1;
    }
}

#[test]
fn driver_completes_multi_step_chain_without_transport_or_tty_knowledge() {
    let phone = token(1);
    let code = token(2);
    let password = token(3);
    let mut driver = LoginDriver::new(
        FakeBroker {
            script: VecDeque::from([
                (
                    DaemonRequest::LoginStatus,
                    status(LoginState::PhoneNumber, Some(&phone)),
                ),
                (
                    DaemonRequest::LoginPrompt {
                        challenge_id: phone.clone(),
                    },
                    prompt(&phone, OwnerLoginPrompt::PhoneNumber),
                ),
                (
                    DaemonRequest::LoginSubmit {
                        challenge_id: phone.clone(),
                        input: phone_input(),
                    },
                    DaemonResponse::LoginSubmitted {
                        challenge_id: phone.clone(),
                    },
                ),
                (
                    DaemonRequest::LoginStatus,
                    status(LoginState::Code, Some(&code)),
                ),
                (
                    DaemonRequest::LoginCodeResend {
                        challenge_id: code.clone(),
                    },
                    DaemonResponse::CommandError {
                        code: telegram_protocol::CommandErrorCode::LoginCodeResendUnavailable,
                    },
                ),
                (
                    DaemonRequest::LoginPrompt {
                        challenge_id: code.clone(),
                    },
                    prompt(&code, OwnerLoginPrompt::AuthenticationCode),
                ),
                (
                    DaemonRequest::LoginSubmit {
                        challenge_id: code.clone(),
                        input: code_input(),
                    },
                    DaemonResponse::LoginSubmitted {
                        challenge_id: code.clone(),
                    },
                ),
                (
                    DaemonRequest::LoginStatus,
                    status(LoginState::Password, Some(&password)),
                ),
                (
                    DaemonRequest::LoginPrompt {
                        challenge_id: password.clone(),
                    },
                    prompt(
                        &password,
                        OwnerLoginPrompt::Password {
                            hint: ProtectedString::new(String::new()),
                            has_recovery_email_address: false,
                            recovery_email_address_pattern: ProtectedString::new(String::new()),
                        },
                    ),
                ),
                (
                    DaemonRequest::LoginSubmit {
                        challenge_id: password.clone(),
                        input: password_input(),
                    },
                    DaemonResponse::LoginSubmitted {
                        challenge_id: password,
                    },
                ),
                (DaemonRequest::LoginStatus, status(LoginState::Ready, None)),
            ]),
        },
        FakePrompt {
            actions: VecDeque::from([
                LoginAction::Submit(phone_input()),
                LoginAction::Submit(code_input()),
                LoginAction::Submit(password_input()),
            ]),
            notices: Vec::new(),
        },
        FakeRuntime::default(),
    );

    assert!(matches!(
        driver.run(None).unwrap(),
        DaemonResponse::LoginStatus {
            state: LoginState::Ready,
            ..
        }
    ));
    assert!(driver.broker.script.is_empty());
}

#[test]
fn one_shot_handoff_stops_after_exact_submission() {
    let challenge = token(4);
    let mut driver = LoginDriver::new(
        FakeBroker {
            script: VecDeque::from([
                (
                    DaemonRequest::LoginStatus,
                    status(LoginState::PhoneNumber, Some(&challenge)),
                ),
                (
                    DaemonRequest::LoginPrompt {
                        challenge_id: challenge.clone(),
                    },
                    prompt(&challenge, OwnerLoginPrompt::PhoneNumber),
                ),
                (
                    DaemonRequest::LoginSubmit {
                        challenge_id: challenge.clone(),
                        input: phone_input(),
                    },
                    DaemonResponse::LoginSubmitted {
                        challenge_id: challenge.clone(),
                    },
                ),
            ]),
        },
        FakePrompt {
            actions: VecDeque::from([LoginAction::Submit(phone_input())]),
            notices: Vec::new(),
        },
        FakeRuntime::default(),
    );

    assert_eq!(
        driver.run(Some(challenge.clone())).unwrap(),
        DaemonResponse::LoginSubmitted {
            challenge_id: challenge
        }
    );
    assert!(driver.broker.script.is_empty());
}

#[test]
fn cancellation_is_checked_before_any_broker_call() {
    let mut driver = LoginDriver::new(
        FakeBroker {
            script: VecDeque::new(),
        },
        FakePrompt {
            actions: VecDeque::new(),
            notices: Vec::new(),
        },
        FakeRuntime {
            cancelled: true,
            waits: 0,
        },
    );
    assert_eq!(
        driver.run(None),
        Err(CliError::new(ClientErrorCode::Cancelled))
    );
}

#[test]
fn mismatched_owner_prompt_fails_closed() {
    let challenge = token(5);
    let other = token(6);
    let mut driver = LoginDriver::new(
        FakeBroker {
            script: VecDeque::from([
                (
                    DaemonRequest::LoginStatus,
                    status(LoginState::PhoneNumber, Some(&challenge)),
                ),
                (
                    DaemonRequest::LoginPrompt {
                        challenge_id: challenge,
                    },
                    prompt(&other, OwnerLoginPrompt::PhoneNumber),
                ),
            ]),
        },
        FakePrompt {
            actions: VecDeque::new(),
            notices: Vec::new(),
        },
        FakeRuntime::default(),
    );
    assert_eq!(
        driver.run(None),
        Err(CliError::new(ClientErrorCode::InvalidResponse))
    );
}

fn token(generation: u64) -> LoginChallengeId {
    LoginChallengeId::new(format!("auth-{:032x}-{generation:016x}", 1))
}

fn status(state: LoginState, challenge: Option<&LoginChallengeId>) -> DaemonResponse {
    DaemonResponse::LoginStatus {
        state,
        challenge_id: challenge.cloned(),
        next_action: match state {
            LoginState::Ready => LoginNextAction::Ready,
            _ => LoginNextAction::SubmitViaProtectedChannel,
        },
    }
}

fn prompt(challenge: &LoginChallengeId, prompt: OwnerLoginPrompt) -> DaemonResponse {
    DaemonResponse::LoginPrompt {
        challenge_id: challenge.clone(),
        prompt,
    }
}

fn phone_input() -> LoginInput {
    LoginInput::PhoneNumber {
        value: ProtectedString::new("+10000000000".to_owned()),
    }
}

fn code_input() -> LoginInput {
    LoginInput::AuthenticationCode {
        value: ProtectedString::new("12345".to_owned()),
    }
}

fn password_input() -> LoginInput {
    LoginInput::Password {
        value: ProtectedString::new("secret".to_owned()),
    }
}
