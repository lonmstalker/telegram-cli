//! Тестируемый owner-login driver; transport, prompt и время приходят через adapters.

use std::sync::atomic::Ordering;
use std::thread;

use telegram_protocol::{
    ClientErrorCode, CommandErrorCode, DaemonRequest, DaemonResponse, LoginChallengeId, LoginInput,
    LoginNextAction, LoginState, OwnerLoginPrompt, ProtectedString,
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

enum DriverStep {
    Continue,
    Return(DaemonResponse),
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
            if !challenge_matches(&response, expected_challenge.as_ref()) {
                return Ok(command_error(CommandErrorCode::LoginChallengeInvalid));
            }
            match self.handle_response(response, expected_challenge.as_ref())? {
                DriverStep::Continue => {}
                DriverStep::Return(response) => return Ok(response),
            }
        }
    }

    fn handle_response(
        &mut self,
        response: DaemonResponse,
        expected_challenge: Option<&LoginChallengeId>,
    ) -> Result<DriverStep, CliError> {
        match response {
            response @ DaemonResponse::LoginStatus {
                state:
                    LoginState::Ready
                    | LoginState::LoggingOut
                    | LoginState::Closing
                    | LoginState::Closed
                    | LoginState::Unknown,
                ..
            } => Ok(DriverStep::Return(response)),
            DaemonResponse::LoginStatus {
                state,
                challenge_id: Some(challenge_id),
                next_action,
            } => self.handle_challenge(state, challenge_id, next_action, expected_challenge),
            DaemonResponse::LoginStatus {
                challenge_id: None, ..
            } => {
                self.runtime.wait();
                Ok(DriverStep::Continue)
            }
            response @ (DaemonResponse::CommandError { .. } | DaemonResponse::Error { .. }) => {
                Ok(DriverStep::Return(response))
            }
            _ => Err(invalid_response()),
        }
    }

    fn handle_challenge(
        &mut self,
        state: LoginState,
        challenge_id: LoginChallengeId,
        next_action: LoginNextAction,
        expected_challenge: Option<&LoginChallengeId>,
    ) -> Result<DriverStep, CliError> {
        if self.waiting_for.as_ref() == Some(&challenge_id) {
            self.runtime.wait();
            return Ok(DriverStep::Continue);
        }
        if let Some(step) =
            self.try_automatic_resend(state, &challenge_id, expected_challenge.is_none())?
        {
            return Ok(step);
        }
        let prompt = self.fetch_prompt(&challenge_id)?;
        let action = self.prompt.action(state, prompt)?;
        self.handle_action(action, state, challenge_id, next_action, expected_challenge)
    }

    fn try_automatic_resend(
        &mut self,
        state: LoginState,
        challenge_id: &LoginChallengeId,
        enabled: bool,
    ) -> Result<Option<DriverStep>, CliError> {
        if !enabled
            || state != LoginState::Code
            || self.automatic_resend_for.as_ref() == Some(challenge_id)
        {
            return Ok(None);
        }
        self.automatic_resend_for = Some(challenge_id.clone());
        match self.resend(challenge_id)? {
            DaemonResponse::LoginCodeResent {
                challenge_id: resent_id,
            } if resent_id == *challenge_id => {
                self.prompt.notice("Запрошен новый код Telegram.\n")?;
                self.waiting_for = Some(challenge_id.clone());
                Ok(Some(DriverStep::Continue))
            }
            DaemonResponse::CommandError {
                code: CommandErrorCode::LoginCodeResendUnavailable,
            } => Ok(None),
            response @ (DaemonResponse::CommandError { .. } | DaemonResponse::Error { .. }) => {
                Ok(Some(DriverStep::Return(response)))
            }
            _ => Err(invalid_response()),
        }
    }

    fn handle_action(
        &mut self,
        action: LoginAction,
        state: LoginState,
        challenge_id: LoginChallengeId,
        next_action: LoginNextAction,
        expected_challenge: Option<&LoginChallengeId>,
    ) -> Result<DriverStep, CliError> {
        match action {
            LoginAction::Submit(input) => {
                self.submit_input(state, challenge_id, input, expected_challenge)
            }
            LoginAction::ResendCode => self.manual_resend(&challenge_id, expected_challenge),
            LoginAction::Wait if expected_challenge.is_some() => {
                Ok(DriverStep::Return(DaemonResponse::LoginStatus {
                    state,
                    challenge_id: Some(challenge_id),
                    next_action,
                }))
            }
            LoginAction::Wait => {
                self.waiting_for = Some(challenge_id);
                self.runtime.wait();
                Ok(DriverStep::Continue)
            }
        }
    }

    fn manual_resend(
        &mut self,
        challenge_id: &LoginChallengeId,
        expected_challenge: Option<&LoginChallengeId>,
    ) -> Result<DriverStep, CliError> {
        match self.resend(challenge_id)? {
            DaemonResponse::LoginCodeResent {
                challenge_id: resent_id,
            } if resent_id == *challenge_id => {
                if expected_challenge.is_some() {
                    Ok(DriverStep::Return(DaemonResponse::LoginCodeResent {
                        challenge_id: resent_id,
                    }))
                } else {
                    self.waiting_for = Some(challenge_id.clone());
                    Ok(DriverStep::Continue)
                }
            }
            response @ (DaemonResponse::CommandError { .. } | DaemonResponse::Error { .. }) => {
                Ok(DriverStep::Return(response))
            }
            _ => Err(invalid_response()),
        }
    }

    fn submit_input(
        &mut self,
        state: LoginState,
        challenge_id: LoginChallengeId,
        input: LoginInput,
        expected_challenge: Option<&LoginChallengeId>,
    ) -> Result<DriverStep, CliError> {
        let response = self.broker.exchange(DaemonRequest::LoginSubmit {
            challenge_id: challenge_id.clone(),
            input,
        })?;
        match response {
            DaemonResponse::LoginSubmitted {
                challenge_id: submitted_id,
            } if submitted_id == challenge_id => {
                if expected_challenge.is_some() {
                    Ok(DriverStep::Return(DaemonResponse::LoginSubmitted {
                        challenge_id: submitted_id,
                    }))
                } else {
                    self.waiting_for = Some(challenge_id);
                    Ok(DriverStep::Continue)
                }
            }
            DaemonResponse::CommandError {
                code: CommandErrorCode::LoginSubmissionRejected,
            } if expected_challenge.is_none() && state == LoginState::Code => {
                self.handle_code_rejection(&challenge_id)
            }
            response @ (DaemonResponse::CommandError { .. } | DaemonResponse::Error { .. }) => {
                Ok(DriverStep::Return(response))
            }
            _ => Err(invalid_response()),
        }
    }

    fn handle_code_rejection(
        &mut self,
        challenge_id: &LoginChallengeId,
    ) -> Result<DriverStep, CliError> {
        match self.resend(challenge_id)? {
            DaemonResponse::LoginCodeResent {
                challenge_id: resent_id,
            } if resent_id == *challenge_id => {
                self.prompt
                    .notice("Код отклонён. Запрошен новый код Telegram.\n")?;
                self.waiting_for = Some(challenge_id.clone());
                Ok(DriverStep::Continue)
            }
            DaemonResponse::CommandError {
                code: CommandErrorCode::LoginCodeResendUnavailable,
            } => {
                self.prompt
                    .notice("Код отклонён Telegram. Введите код ещё раз.\n")?;
                self.waiting_for = None;
                Ok(DriverStep::Continue)
            }
            response @ (DaemonResponse::CommandError { .. } | DaemonResponse::Error { .. }) => {
                Ok(DriverStep::Return(response))
            }
            _ => Err(invalid_response()),
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
    if !prompt_matches_state(state, &prompt) {
        return Err(invalid_response());
    }
    match prompt {
        OwnerLoginPrompt::PhoneNumber | OwnerLoginPrompt::PremiumPurchase => phone_login_action(),
        OwnerLoginPrompt::AuthenticationCode => authentication_code_login_action(),
        OwnerLoginPrompt::Password {
            hint,
            has_recovery_email_address,
            recovery_email_address_pattern,
        } => password_login_action(
            hint,
            has_recovery_email_address,
            recovery_email_address_pattern,
        ),
        OwnerLoginPrompt::EmailAddress {
            allow_apple_id,
            allow_google_id,
        } => email_address_login_action(allow_apple_id, allow_google_id),
        OwnerLoginPrompt::EmailCode {
            allow_apple_id,
            allow_google_id,
        } => email_code_login_action(allow_apple_id, allow_google_id),
        OwnerLoginPrompt::Registration {
            terms,
            minimum_user_age,
            show_popup,
        } => registration_login_action(terms, minimum_user_age, show_popup),
        OwnerLoginPrompt::QrCode { link } => qr_login_action(link),
    }
}

fn authentication_code_login_action() -> Result<LoginAction, CliError> {
    Ok(LoginAction::Submit(LoginInput::AuthenticationCode {
        value: read_tty_visible("Код Telegram: ")?,
    }))
}

fn email_address_login_action(
    allow_apple_id: bool,
    allow_google_id: bool,
) -> Result<LoginAction, CliError> {
    Ok(LoginAction::Submit(email_login_input(
        allow_apple_id,
        allow_google_id,
        false,
    )?))
}

fn email_code_login_action(
    allow_apple_id: bool,
    allow_google_id: bool,
) -> Result<LoginAction, CliError> {
    if read_yes_no("Запросить новый email-код? [y/N]: ")? {
        return Ok(LoginAction::ResendCode);
    }
    Ok(LoginAction::Submit(email_login_input(
        allow_apple_id,
        allow_google_id,
        true,
    )?))
}

fn prompt_matches_state(state: LoginState, prompt: &OwnerLoginPrompt) -> bool {
    matches!(
        (state, prompt),
        (LoginState::PhoneNumber, OwnerLoginPrompt::PhoneNumber)
            | (
                LoginState::PremiumPurchase,
                OwnerLoginPrompt::PremiumPurchase
            )
            | (LoginState::Code, OwnerLoginPrompt::AuthenticationCode)
            | (LoginState::Password, OwnerLoginPrompt::Password { .. })
            | (
                LoginState::EmailAddress,
                OwnerLoginPrompt::EmailAddress { .. }
            )
            | (LoginState::EmailCode, OwnerLoginPrompt::EmailCode { .. })
            | (
                LoginState::Registration,
                OwnerLoginPrompt::Registration { .. }
            )
            | (LoginState::QrCode, OwnerLoginPrompt::QrCode { .. })
    )
}

fn phone_login_action() -> Result<LoginAction, CliError> {
    let input = if read_yes_no("Войти по QR вместо номера? [y/N]: ")? {
        LoginInput::QrCode
    } else {
        LoginInput::PhoneNumber {
            value: read_tty_visible("Телефон: ")?,
        }
    };
    Ok(LoginAction::Submit(input))
}

fn password_login_action(
    hint: ProtectedString,
    has_recovery_email_address: bool,
    recovery_email_address_pattern: ProtectedString,
) -> Result<LoginAction, CliError> {
    let hint = hint.into_inner();
    if !hint.is_empty() {
        write_tty_notice(&format!("Подсказка Telegram: {hint}\n"))?;
    }
    if has_recovery_email_address {
        let pattern = recovery_email_address_pattern.into_inner();
        write_tty_notice(&format!("Recovery email: {pattern}\n"))?;
    }
    Ok(LoginAction::Submit(LoginInput::Password {
        value: read_cloud_password()?,
    }))
}

fn registration_login_action(
    terms: ProtectedString,
    minimum_user_age: i32,
    show_popup: bool,
) -> Result<LoginAction, CliError> {
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
    Ok(LoginAction::Submit(LoginInput::Registration {
        first_name: read_tty_visible("Имя: ")?,
        last_name: read_tty_visible("Фамилия (можно пустую): ")?,
        terms_accepted: true,
        disable_notification: !notify_contacts,
    }))
}

fn qr_login_action(link: ProtectedString) -> Result<LoginAction, CliError> {
    let link = link.into_inner();
    write_tty_notice("Откройте эту ссылку на уже авторизованном устройстве Telegram:\n")?;
    write_tty_notice(&link)?;
    write_tty_notice("\n")?;
    Ok(LoginAction::Wait)
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

fn challenge_matches(response: &DaemonResponse, expected: Option<&LoginChallengeId>) -> bool {
    match (expected, response) {
        (Some(expected), DaemonResponse::LoginStatus { challenge_id, .. }) => {
            challenge_id.as_ref() == Some(expected)
        }
        _ => true,
    }
}

#[cfg(test)]
mod tests;
