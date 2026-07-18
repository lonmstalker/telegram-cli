//! Тестируемый owner-login driver; transport, prompt и время приходят через adapters.

use std::sync::atomic::Ordering;
use std::thread;

use telegram_protocol::{
    ClientErrorCode, CommandErrorCode, DaemonRequest, DaemonResponse, LoginChallengeId, LoginInput,
    LoginState, OwnerLoginPrompt,
};
use zeroize::Zeroize;

use crate::{
    CliError, RECEIVED_SIGNAL, WATCH_POLL_INTERVAL, exchange, read_cloud_password,
    read_tty_visible, read_yes_no, write_tty_notice,
};

pub(crate) fn run(
    profile: &str,
    expected_challenge: Option<LoginChallengeId>,
) -> Result<DaemonResponse, CliError> {
    LoginDriver::new(
        SocketLoginBroker { profile },
        TtyOwnerPrompt,
        SystemLoginRuntime,
    )
    .run(expected_challenge)
}

trait LoginBroker {
    fn exchange(&mut self, request: DaemonRequest) -> Result<DaemonResponse, CliError>;
}

trait OwnerPrompt {
    fn action(
        &mut self,
        state: LoginState,
        prompt: OwnerLoginPrompt,
    ) -> Result<LoginAction, CliError>;

    fn notice(&mut self, message: &'static str) -> Result<(), CliError>;
}

trait LoginRuntime {
    fn cancelled(&self) -> bool;
    fn wait(&mut self);
}

enum LoginAction {
    Submit(LoginInput),
    ResendCode,
    Wait,
}

struct LoginDriver<B, P, R> {
    broker: B,
    prompt: P,
    runtime: R,
    waiting_for: Option<LoginChallengeId>,
    automatic_resend_for: Option<LoginChallengeId>,
}

impl<B, P, R> LoginDriver<B, P, R>
where
    B: LoginBroker,
    P: OwnerPrompt,
    R: LoginRuntime,
{
    fn new(broker: B, prompt: P, runtime: R) -> Self {
        Self {
            broker,
            prompt,
            runtime,
            waiting_for: None,
            automatic_resend_for: None,
        }
    }

    fn run(
        &mut self,
        expected_challenge: Option<LoginChallengeId>,
    ) -> Result<DaemonResponse, CliError> {
        loop {
            if self.runtime.cancelled() {
                return Err(CliError::new(ClientErrorCode::Cancelled));
            }
            let response = self.broker.exchange(DaemonRequest::LoginStatus)?;
            if let (Some(expected), DaemonResponse::LoginStatus { challenge_id, .. }) =
                (expected_challenge.as_ref(), &response)
                && challenge_id.as_ref() != Some(expected)
            {
                return Ok(command_error(CommandErrorCode::LoginChallengeInvalid));
            }
            match response {
                response @ DaemonResponse::LoginStatus {
                    state: LoginState::Ready,
                    ..
                }
                | response @ DaemonResponse::LoginStatus {
                    state:
                        LoginState::LoggingOut
                        | LoginState::Closing
                        | LoginState::Closed
                        | LoginState::Unknown,
                    ..
                } => return Ok(response),
                ref response @ DaemonResponse::LoginStatus {
                    state,
                    challenge_id: Some(ref challenge_id),
                    ..
                } => {
                    if self.waiting_for.as_ref() == Some(challenge_id) {
                        self.runtime.wait();
                        continue;
                    }
                    if expected_challenge.is_none()
                        && state == LoginState::Code
                        && self.automatic_resend_for.as_ref() != Some(challenge_id)
                    {
                        self.automatic_resend_for = Some(challenge_id.clone());
                        match self.resend(challenge_id)? {
                            DaemonResponse::LoginCodeResent {
                                challenge_id: resent_id,
                            } if resent_id == *challenge_id => {
                                self.prompt.notice("Запрошен новый код Telegram.\n")?;
                                self.waiting_for = Some(challenge_id.clone());
                                continue;
                            }
                            DaemonResponse::CommandError {
                                code: CommandErrorCode::LoginCodeResendUnavailable,
                            } => {}
                            response @ (DaemonResponse::CommandError { .. }
                            | DaemonResponse::Error { .. }) => return Ok(response),
                            _ => return Err(invalid_response()),
                        }
                    }
                    let prompt = self.fetch_prompt(challenge_id)?;
                    let input = match self.prompt.action(state, prompt)? {
                        LoginAction::Submit(input) => input,
                        LoginAction::ResendCode => {
                            let response = self.resend(challenge_id)?;
                            match response {
                                ref response @ DaemonResponse::LoginCodeResent {
                                    challenge_id: ref resent_id,
                                } if resent_id == challenge_id => {
                                    if expected_challenge.is_some() {
                                        return Ok(response.clone());
                                    }
                                    self.waiting_for = Some(challenge_id.clone());
                                    continue;
                                }
                                response @ (DaemonResponse::CommandError { .. }
                                | DaemonResponse::Error { .. }) => return Ok(response),
                                _ => return Err(invalid_response()),
                            }
                        }
                        LoginAction::Wait => {
                            if expected_challenge.is_some() {
                                return Ok(response.clone());
                            }
                            self.waiting_for = Some(challenge_id.clone());
                            self.runtime.wait();
                            continue;
                        }
                    };
                    let submitted = self.broker.exchange(DaemonRequest::LoginSubmit {
                        challenge_id: challenge_id.clone(),
                        input,
                    })?;
                    match submitted {
                        ref response @ DaemonResponse::LoginSubmitted {
                            challenge_id: ref submitted_id,
                        } if submitted_id == challenge_id => {
                            if expected_challenge.is_some() {
                                return Ok(response.clone());
                            }
                            self.waiting_for = Some(challenge_id.clone());
                        }
                        DaemonResponse::CommandError {
                            code: CommandErrorCode::LoginSubmissionRejected,
                        } if expected_challenge.is_none() && state == LoginState::Code => {
                            match self.resend(challenge_id)? {
                                DaemonResponse::LoginCodeResent {
                                    challenge_id: resent_id,
                                } if resent_id == *challenge_id => {
                                    self.prompt
                                        .notice("Код отклонён. Запрошен новый код Telegram.\n")?;
                                    self.waiting_for = Some(challenge_id.clone());
                                }
                                DaemonResponse::CommandError {
                                    code: CommandErrorCode::LoginCodeResendUnavailable,
                                } => {
                                    self.prompt
                                        .notice("Код отклонён Telegram. Введите код ещё раз.\n")?;
                                    self.waiting_for = None;
                                }
                                response @ (DaemonResponse::CommandError { .. }
                                | DaemonResponse::Error { .. }) => return Ok(response),
                                _ => return Err(invalid_response()),
                            }
                        }
                        response @ (DaemonResponse::CommandError { .. }
                        | DaemonResponse::Error { .. }) => return Ok(response),
                        _ => return Err(invalid_response()),
                    }
                }
                DaemonResponse::LoginStatus {
                    challenge_id: None, ..
                } => self.runtime.wait(),
                response @ (DaemonResponse::CommandError { .. } | DaemonResponse::Error { .. }) => {
                    return Ok(response);
                }
                _ => return Err(invalid_response()),
            }
        }
    }

    fn fetch_prompt(
        &mut self,
        challenge_id: &LoginChallengeId,
    ) -> Result<OwnerLoginPrompt, CliError> {
        match self.broker.exchange(DaemonRequest::LoginPrompt {
            challenge_id: challenge_id.clone(),
        })? {
            DaemonResponse::LoginPrompt {
                challenge_id: returned,
                prompt,
            } if returned == *challenge_id => Ok(prompt),
            _ => Err(invalid_response()),
        }
    }

    fn resend(&mut self, challenge_id: &LoginChallengeId) -> Result<DaemonResponse, CliError> {
        self.broker.exchange(DaemonRequest::LoginCodeResend {
            challenge_id: challenge_id.clone(),
        })
    }
}

struct SocketLoginBroker<'a> {
    profile: &'a str,
}

impl LoginBroker for SocketLoginBroker<'_> {
    fn exchange(&mut self, request: DaemonRequest) -> Result<DaemonResponse, CliError> {
        exchange(self.profile, &request)
    }
}

struct TtyOwnerPrompt;

impl OwnerPrompt for TtyOwnerPrompt {
    fn action(
        &mut self,
        state: LoginState,
        prompt: OwnerLoginPrompt,
    ) -> Result<LoginAction, CliError> {
        tty_login_action(state, prompt)
    }

    fn notice(&mut self, message: &'static str) -> Result<(), CliError> {
        write_tty_notice(message)
    }
}

struct SystemLoginRuntime;

impl LoginRuntime for SystemLoginRuntime {
    fn cancelled(&self) -> bool {
        RECEIVED_SIGNAL.load(Ordering::Relaxed) != 0
    }

    fn wait(&mut self) {
        thread::sleep(WATCH_POLL_INTERVAL);
    }
}

fn tty_login_action(state: LoginState, prompt: OwnerLoginPrompt) -> Result<LoginAction, CliError> {
    let input = match (state, prompt) {
        (LoginState::PhoneNumber, OwnerLoginPrompt::PhoneNumber)
        | (LoginState::PremiumPurchase, OwnerLoginPrompt::PremiumPurchase) => {
            if read_yes_no("Войти по QR вместо номера? [y/N]: ")? {
                LoginInput::QrCode
            } else {
                LoginInput::PhoneNumber {
                    value: read_tty_visible("Телефон: ")?,
                }
            }
        }
        (LoginState::Code, OwnerLoginPrompt::AuthenticationCode) => {
            LoginInput::AuthenticationCode {
                value: read_tty_visible("Код Telegram: ")?,
            }
        }
        (
            LoginState::Password,
            OwnerLoginPrompt::Password {
                hint,
                has_recovery_email_address,
                recovery_email_address_pattern,
            },
        ) => {
            let hint = hint.into_inner();
            if !hint.is_empty() {
                write_tty_notice(&format!("Подсказка Telegram: {hint}\n"))?;
            }
            if has_recovery_email_address {
                let pattern = recovery_email_address_pattern.into_inner();
                write_tty_notice(&format!("Recovery email: {pattern}\n"))?;
            }
            LoginInput::Password {
                value: read_cloud_password()?,
            }
        }
        (
            LoginState::EmailAddress,
            OwnerLoginPrompt::EmailAddress {
                allow_apple_id,
                allow_google_id,
            },
        ) => email_login_input(allow_apple_id, allow_google_id, false)?,
        (
            LoginState::EmailCode,
            OwnerLoginPrompt::EmailCode {
                allow_apple_id,
                allow_google_id,
            },
        ) => {
            if read_yes_no("Запросить новый email-код? [y/N]: ")? {
                return Ok(LoginAction::ResendCode);
            }
            email_login_input(allow_apple_id, allow_google_id, true)?
        }
        (
            LoginState::Registration,
            OwnerLoginPrompt::Registration {
                terms,
                minimum_user_age,
                show_popup,
            },
        ) => {
            let terms = terms.into_inner();
            write_tty_notice("Условия использования Telegram:\n")?;
            write_tty_notice(&terms)?;
            write_tty_notice("\n")?;
            if minimum_user_age > 0 {
                write_tty_notice(&format!("Минимальный возраст: {minimum_user_age}.\n"))?;
            }
            if show_popup {
                write_tty_notice("Telegram требует явного подтверждения этих условий.\n")?;
            }
            if !read_yes_no("Принять условия использования? [y/N]: ")? {
                return Err(CliError::new(ClientErrorCode::Cancelled));
            }
            let notify_contacts =
                read_yes_no("Уведомить контакты, которые добавили вас, о регистрации? [y/N]: ")?;
            LoginInput::Registration {
                first_name: read_tty_visible("Имя: ")?,
                last_name: read_tty_visible("Фамилия (можно пустую): ")?,
                terms_accepted: true,
                disable_notification: !notify_contacts,
            }
        }
        (LoginState::QrCode, OwnerLoginPrompt::QrCode { link }) => {
            let link = link.into_inner();
            write_tty_notice("Откройте эту ссылку на уже авторизованном устройстве Telegram:\n")?;
            write_tty_notice(&link)?;
            write_tty_notice("\n")?;
            return Ok(LoginAction::Wait);
        }
        _ => return Err(invalid_response()),
    };
    Ok(LoginAction::Submit(input))
}

fn email_login_input(
    allow_apple_id: bool,
    allow_google_id: bool,
    code_state: bool,
) -> Result<LoginInput, CliError> {
    if allow_apple_id || allow_google_id {
        loop {
            let mut choice =
                read_tty_visible("Способ: email [e], Apple ID [a], Google ID [g]: ")?.into_inner();
            let selected = choice.trim().to_ascii_lowercase();
            choice.zeroize();
            match selected.as_str() {
                "" | "e" | "email" => break,
                "a" | "apple" if allow_apple_id => {
                    return Ok(LoginInput::AppleIdToken {
                        value: read_tty_visible("Apple ID token: ")?,
                    });
                }
                "g" | "google" if allow_google_id => {
                    return Ok(LoginInput::GoogleIdToken {
                        value: read_tty_visible("Google ID token: ")?,
                    });
                }
                _ => write_tty_notice("Этот способ недоступен; выберите e, a или g.\n")?,
            }
        }
    }
    if code_state {
        Ok(LoginInput::EmailCode {
            value: read_tty_visible("Код из email: ")?,
        })
    } else {
        Ok(LoginInput::EmailAddress {
            value: read_tty_visible("Email: ")?,
        })
    }
}

fn invalid_response() -> CliError {
    CliError::new(ClientErrorCode::InvalidResponse)
}

fn command_error(code: CommandErrorCode) -> DaemonResponse {
    DaemonResponse::CommandError { code }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use telegram_protocol::{
        ClientErrorCode, DaemonRequest, DaemonResponse, LoginChallengeId, LoginInput,
        LoginNextAction, LoginState, OwnerLoginPrompt, ProtectedString,
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
}
