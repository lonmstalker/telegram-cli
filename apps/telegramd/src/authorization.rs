//! Единый daemon-owned authorization coordinator.

use std::time::{Duration, Instant, SystemTime};

use serde_json::Value;
use telegram_core::authorization::{
    AuthorizationChallengeKind, AuthorizationError, AuthorizationInput, AuthorizationMachine,
    AuthorizationRequest, AuthorizationStep, ChallengeId, SensitiveString, SubmissionOutcome,
};
use telegram_core::database_key::{DatabaseKey, TdlibParameters};
use telegram_core::registry::AccountKind;
use telegram_core::runtime::CoreRuntime;
use telegram_core::transport::TransportError;
use telegram_protocol::{
    CommandErrorCode, DaemonResponse, LoginChallengeId, LoginInput, LoginState, OwnerLoginPrompt,
    ProtectedString,
};

const AUTH_CALL_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationObservation {
    ParametersRequired { generation: ChallengeId },
    InteractiveRequired,
    ReadyObserved,
    LoggingOut,
    Closing,
    Closed,
}

/// Единственный auth-owner внутри daemon: от startup parameters до verified re-auth Ready.
pub struct AuthorizationCoordinator {
    machine: AuthorizationMachine,
    step: Option<AuthorizationStep>,
    observed_at: Option<Instant>,
    epoch: u128,
    challenge_id: Option<LoginChallengeId>,
    uncertain_submission: Option<(u64, ChallengeId)>,
    verified_account: Option<AccountKind>,
}

impl Default for AuthorizationCoordinator {
    fn default() -> Self {
        let epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            ^ ((std::process::id() as u128) << 64);
        Self::with_epoch(epoch)
    }
}

impl AuthorizationCoordinator {
    pub(crate) fn with_epoch(epoch: u128) -> Self {
        Self {
            machine: AuthorizationMachine::default(),
            step: None,
            observed_at: None,
            epoch,
            challenge_id: None,
            uncertain_submission: None,
            verified_account: None,
        }
    }

    pub fn observe(
        &mut self,
        state: &Value,
        now: Instant,
    ) -> Result<AuthorizationObservation, AuthorizationError> {
        let step = self.machine.observe_state(state)?;
        let observation = observation(&step);
        self.step = Some(step.clone());
        self.observed_at = Some(now);
        self.uncertain_submission = None;
        self.verified_account = None;
        self.challenge_id = challenge_token(self.epoch, &step);
        Ok(observation)
    }

    pub fn status(&self) -> (LoginState, Option<LoginChallengeId>) {
        let Some(step) = &self.step else {
            return (LoginState::Unknown, None);
        };
        (login_state(step), self.challenge_id.clone())
    }

    pub fn prompt(
        &self,
        challenge_id: &LoginChallengeId,
    ) -> Result<OwnerLoginPrompt, AuthorizationError> {
        self.require_current_token(challenge_id)?;
        let Some(AuthorizationStep::Challenge(challenge)) = &self.step else {
            return Err(AuthorizationError::InputDoesNotMatchState);
        };
        Ok(owner_prompt(&challenge.kind))
    }

    pub fn submit_parameters(
        &mut self,
        generation: ChallengeId,
        parameters: TdlibParameters,
        key: &DatabaseKey,
    ) -> Result<AuthorizationRequest, AuthorizationError> {
        self.machine.submit_parameters(generation, parameters, key)
    }

    pub fn parameters_failed(
        &mut self,
        generation: ChallengeId,
        tdlib_error_code: i32,
    ) -> Result<(), AuthorizationError> {
        self.machine.parameters_failed(generation, tdlib_error_code)
    }

    pub fn mark_identity_verified(
        &mut self,
        account: AccountKind,
    ) -> Result<(), AuthorizationError> {
        if !matches!(self.step, Some(AuthorizationStep::Ready)) {
            return Err(AuthorizationError::InputDoesNotMatchState);
        }
        self.verified_account = Some(account);
        Ok(())
    }

    pub fn is_identity_verified(&self) -> bool {
        self.verified_account.is_some()
    }

    pub fn account_kind(&self) -> Option<AccountKind> {
        self.verified_account
    }

    pub fn submit(
        &mut self,
        runtime: Option<&mut CoreRuntime>,
        challenge_id: &LoginChallengeId,
        input: LoginInput,
        now: Instant,
    ) -> DaemonResponse {
        let (challenge, request) = match self.begin_submit(challenge_id, input) {
            Ok(request) => request,
            Err(AuthorizationError::SubmissionPending) => {
                return command_error(CommandErrorCode::LoginSubmissionPending);
            }
            Err(_) => return command_error(CommandErrorCode::LoginChallengeInvalid),
        };
        self.dispatch(
            runtime,
            challenge_id.clone(),
            challenge,
            request,
            now,
            DispatchKind::Submit,
        )
    }

    pub fn resend_code(
        &mut self,
        runtime: Option<&mut CoreRuntime>,
        challenge_id: &LoginChallengeId,
        now: Instant,
    ) -> DaemonResponse {
        let (challenge, request) = match self.begin_resend(challenge_id, now) {
            Ok(request) => request,
            Err(AuthorizationError::SubmissionPending) => {
                return command_error(CommandErrorCode::LoginSubmissionPending);
            }
            Err(AuthorizationError::CodeResendUnavailable) => {
                return command_error(CommandErrorCode::LoginCodeResendUnavailable);
            }
            Err(_) => return command_error(CommandErrorCode::LoginChallengeInvalid),
        };
        self.dispatch(
            runtime,
            challenge_id.clone(),
            challenge,
            request,
            now,
            DispatchKind::Resend,
        )
    }

    pub fn reconcile_response(
        &mut self,
        correlation_id: u64,
        response: &Value,
    ) -> Result<(), AuthorizationError> {
        let Some((expected, challenge)) = self.uncertain_submission else {
            return Ok(());
        };
        if expected != correlation_id {
            return Ok(());
        }
        if response.get("@type").and_then(Value::as_str) != Some("ok") {
            self.machine
                .submission_outcome(challenge, SubmissionOutcome::DefinitiveRejected)?;
            self.uncertain_submission = None;
        }
        Ok(())
    }

    fn begin_submit(
        &mut self,
        challenge_id: &LoginChallengeId,
        input: LoginInput,
    ) -> Result<(ChallengeId, AuthorizationRequest), AuthorizationError> {
        self.require_current_token(challenge_id)?;
        let challenge = self.current_challenge()?;
        let request = self.machine.submit(challenge, authorization_input(input))?;
        Ok((challenge, request))
    }

    fn begin_resend(
        &mut self,
        challenge_id: &LoginChallengeId,
        now: Instant,
    ) -> Result<(ChallengeId, AuthorizationRequest), AuthorizationError> {
        self.require_current_token(challenge_id)?;
        let challenge = match &self.step {
            Some(AuthorizationStep::Challenge(challenge)) => challenge,
            _ => return Err(AuthorizationError::StaleChallenge),
        };
        if let AuthorizationChallengeKind::AuthenticationCode(info) = &challenge.kind {
            let observed_at = self
                .observed_at
                .ok_or(AuthorizationError::CodeResendUnavailable)?;
            let timeout = Duration::from_secs(info.timeout_seconds.max(0) as u64);
            if info.next_delivery_type.is_none()
                || now.checked_duration_since(observed_at).unwrap_or_default() < timeout
            {
                return Err(AuthorizationError::CodeResendUnavailable);
            }
        } else if !matches!(challenge.kind, AuthorizationChallengeKind::EmailCode { .. }) {
            return Err(AuthorizationError::InputDoesNotMatchState);
        }
        let request = self.machine.resend_code(challenge.id)?;
        Ok((challenge.id, request))
    }

    fn require_current_token(
        &self,
        challenge_id: &LoginChallengeId,
    ) -> Result<(), AuthorizationError> {
        if self.challenge_id.as_ref() == Some(challenge_id) {
            Ok(())
        } else {
            Err(AuthorizationError::StaleChallenge)
        }
    }

    fn current_challenge(&self) -> Result<ChallengeId, AuthorizationError> {
        match &self.step {
            Some(AuthorizationStep::Challenge(challenge)) => Ok(challenge.id),
            _ => Err(AuthorizationError::StaleChallenge),
        }
    }

    fn dispatch(
        &mut self,
        runtime: Option<&mut CoreRuntime>,
        token: LoginChallengeId,
        challenge: ChallengeId,
        request: AuthorizationRequest,
        now: Instant,
        kind: DispatchKind,
    ) -> DaemonResponse {
        let Some(runtime) = runtime else {
            if self
                .record_outcome(challenge, SubmissionOutcome::NotSent)
                .is_err()
            {
                return command_error(CommandErrorCode::UnexpectedTdlibResult);
            }
            return command_error(CommandErrorCode::RuntimeUnavailable);
        };
        let deadline = now.checked_add(AUTH_CALL_TIMEOUT).unwrap_or(now);
        let pending = match runtime.transport().request(request.into_value()) {
            Ok(pending) => pending,
            Err(_) => {
                if self
                    .record_outcome(challenge, SubmissionOutcome::NotSent)
                    .is_err()
                {
                    return command_error(CommandErrorCode::UnexpectedTdlibResult);
                }
                return command_error(CommandErrorCode::TdlibTransport);
            }
        };
        let correlation_id = pending.correlation_id();
        match pending.wait_until(deadline) {
            Ok(response) if response.get("@type").and_then(Value::as_str) == Some("ok") => {
                kind.success(token)
            }
            Ok(response) => {
                if self
                    .record_outcome(challenge, SubmissionOutcome::DefinitiveRejected)
                    .is_err()
                {
                    return command_error(CommandErrorCode::UnexpectedTdlibResult);
                }
                command_error(kind.rejection_code(&response))
            }
            Err(error) => {
                if self
                    .record_outcome(challenge, SubmissionOutcome::Uncertain)
                    .is_err()
                {
                    return command_error(CommandErrorCode::UnexpectedTdlibResult);
                }
                self.uncertain_submission = Some((correlation_id, challenge));
                command_error(transport_error_code(error))
            }
        }
    }

    fn record_outcome(
        &mut self,
        challenge: ChallengeId,
        outcome: SubmissionOutcome,
    ) -> Result<(), AuthorizationError> {
        self.machine.submission_outcome(challenge, outcome)
    }
}

#[derive(Clone, Copy)]
enum DispatchKind {
    Submit,
    Resend,
}

impl DispatchKind {
    fn success(self, challenge_id: LoginChallengeId) -> DaemonResponse {
        match self {
            Self::Submit => DaemonResponse::LoginSubmitted { challenge_id },
            Self::Resend => DaemonResponse::LoginCodeResent { challenge_id },
        }
    }

    fn rejection_code(self, response: &Value) -> CommandErrorCode {
        match self {
            Self::Submit => login_submission_error(response),
            Self::Resend => CommandErrorCode::LoginCodeResendRejected,
        }
    }
}

fn challenge_token(epoch: u128, step: &AuthorizationStep) -> Option<LoginChallengeId> {
    let generation = match step {
        AuthorizationStep::ParametersRequired { generation } => *generation,
        AuthorizationStep::Challenge(challenge) => challenge.id,
        AuthorizationStep::Ready
        | AuthorizationStep::LoggingOut
        | AuthorizationStep::Closing
        | AuthorizationStep::Closed => return None,
    };
    Some(LoginChallengeId::new(format!(
        "auth-{epoch:032x}-{:016x}",
        generation.get()
    )))
}

fn observation(step: &AuthorizationStep) -> AuthorizationObservation {
    match step {
        AuthorizationStep::ParametersRequired { generation } => {
            AuthorizationObservation::ParametersRequired {
                generation: *generation,
            }
        }
        AuthorizationStep::Challenge(_) => AuthorizationObservation::InteractiveRequired,
        AuthorizationStep::Ready => AuthorizationObservation::ReadyObserved,
        AuthorizationStep::LoggingOut => AuthorizationObservation::LoggingOut,
        AuthorizationStep::Closing => AuthorizationObservation::Closing,
        AuthorizationStep::Closed => AuthorizationObservation::Closed,
    }
}

fn owner_prompt(kind: &AuthorizationChallengeKind) -> OwnerLoginPrompt {
    match kind {
        AuthorizationChallengeKind::PhoneNumber => OwnerLoginPrompt::PhoneNumber,
        AuthorizationChallengeKind::PremiumPurchase { .. } => OwnerLoginPrompt::PremiumPurchase,
        AuthorizationChallengeKind::AuthenticationCode(_) => OwnerLoginPrompt::AuthenticationCode,
        AuthorizationChallengeKind::Password {
            password_hint,
            has_recovery_email_address,
            recovery_email_address_pattern,
            ..
        } => OwnerLoginPrompt::Password {
            hint: ProtectedString::new(password_hint.expose_secret().to_owned()),
            has_recovery_email_address: *has_recovery_email_address,
            recovery_email_address_pattern: ProtectedString::new(
                recovery_email_address_pattern.expose_secret().to_owned(),
            ),
        },
        AuthorizationChallengeKind::EmailAddress {
            allow_apple_id,
            allow_google_id,
        } => OwnerLoginPrompt::EmailAddress {
            allow_apple_id: *allow_apple_id,
            allow_google_id: *allow_google_id,
        },
        AuthorizationChallengeKind::EmailCode {
            allow_apple_id,
            allow_google_id,
            ..
        } => OwnerLoginPrompt::EmailCode {
            allow_apple_id: *allow_apple_id,
            allow_google_id: *allow_google_id,
        },
        AuthorizationChallengeKind::OtherDeviceConfirmation { link } => OwnerLoginPrompt::QrCode {
            link: ProtectedString::new(link.expose_secret().to_owned()),
        },
        AuthorizationChallengeKind::Registration(terms) => OwnerLoginPrompt::Registration {
            terms: ProtectedString::new(terms.text.clone()),
            minimum_user_age: terms.minimum_user_age,
            show_popup: terms.show_popup,
        },
    }
}

fn login_state(step: &AuthorizationStep) -> LoginState {
    match step {
        AuthorizationStep::ParametersRequired { .. } => LoginState::Parameters,
        AuthorizationStep::Ready => LoginState::Ready,
        AuthorizationStep::LoggingOut => LoginState::LoggingOut,
        AuthorizationStep::Closing => LoginState::Closing,
        AuthorizationStep::Closed => LoginState::Closed,
        AuthorizationStep::Challenge(challenge) => match &challenge.kind {
            AuthorizationChallengeKind::PhoneNumber => LoginState::PhoneNumber,
            AuthorizationChallengeKind::PremiumPurchase { .. } => LoginState::PremiumPurchase,
            AuthorizationChallengeKind::EmailAddress { .. } => LoginState::EmailAddress,
            AuthorizationChallengeKind::EmailCode { .. } => LoginState::EmailCode,
            AuthorizationChallengeKind::AuthenticationCode(_) => LoginState::Code,
            AuthorizationChallengeKind::OtherDeviceConfirmation { .. } => LoginState::QrCode,
            AuthorizationChallengeKind::Registration(_) => LoginState::Registration,
            AuthorizationChallengeKind::Password { .. } => LoginState::Password,
        },
    }
}

fn authorization_input(input: LoginInput) -> AuthorizationInput {
    match input {
        LoginInput::PhoneNumber { value } => {
            AuthorizationInput::PhoneNumber(SensitiveString::new(value.into_inner()))
        }
        LoginInput::QrCode => AuthorizationInput::QrCode {
            other_user_ids: Vec::new(),
        },
        LoginInput::AuthenticationCode { value } => {
            AuthorizationInput::AuthenticationCode(SensitiveString::new(value.into_inner()))
        }
        LoginInput::Password { value } => {
            AuthorizationInput::Password(SensitiveString::new(value.into_inner()))
        }
        LoginInput::EmailAddress { value } => {
            AuthorizationInput::EmailAddress(SensitiveString::new(value.into_inner()))
        }
        LoginInput::EmailCode { value } => {
            AuthorizationInput::EmailCode(SensitiveString::new(value.into_inner()))
        }
        LoginInput::AppleIdToken { value } => {
            AuthorizationInput::AppleIdToken(SensitiveString::new(value.into_inner()))
        }
        LoginInput::GoogleIdToken { value } => {
            AuthorizationInput::GoogleIdToken(SensitiveString::new(value.into_inner()))
        }
        LoginInput::Registration {
            first_name,
            last_name,
            terms_accepted,
            disable_notification,
        } => AuthorizationInput::Registration {
            first_name: SensitiveString::new(first_name.into_inner()),
            last_name: SensitiveString::new(last_name.into_inner()),
            terms_accepted,
            disable_notification,
        },
    }
}

fn login_submission_error(response: &Value) -> CommandErrorCode {
    if response.get("code").and_then(Value::as_i64) == Some(400) {
        CommandErrorCode::LoginSubmissionRejected
    } else {
        CommandErrorCode::TdlibTransport
    }
}

fn transport_error_code(error: TransportError) -> CommandErrorCode {
    match error {
        TransportError::ResponseTimeout
        | TransportError::Backend(_)
        | TransportError::InvalidTdJsonResponse
        | TransportError::TransportStopped => CommandErrorCode::TdlibTransport,
        TransportError::RequestMustBeObject
        | TransportError::ReservedExtra
        | TransportError::CorrelationExhausted => CommandErrorCode::UnexpectedTdlibResult,
    }
}

fn command_error(code: CommandErrorCode) -> DaemonResponse {
    DaemonResponse::CommandError { code }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use serde_json::json;
    use telegram_core::authorization::{AuthorizationError, SubmissionOutcome};
    use telegram_core::registry::AccountKind;
    use telegram_protocol::{
        CommandErrorCode, DaemonResponse, LoginInput, LoginState, ProtectedString,
    };

    use super::AuthorizationCoordinator;

    #[test]
    fn verified_identity_belongs_only_to_the_current_ready_observation() {
        let now = Instant::now();
        let mut coordinator = AuthorizationCoordinator::with_epoch(7);
        coordinator
            .observe(&json!({"@type": "authorizationStateWaitPhoneNumber"}), now)
            .unwrap();
        assert!(!coordinator.is_identity_verified());
        assert_eq!(coordinator.account_kind(), None);
        assert_eq!(
            coordinator.mark_identity_verified(AccountKind::RegularUser),
            Err(AuthorizationError::InputDoesNotMatchState)
        );

        coordinator
            .observe(&json!({"@type": "authorizationStateReady"}), now)
            .unwrap();
        coordinator
            .mark_identity_verified(AccountKind::RegularUser)
            .unwrap();
        assert!(coordinator.is_identity_verified());
        assert_eq!(coordinator.account_kind(), Some(AccountKind::RegularUser));

        coordinator
            .observe(
                &json!({
                    "@type": "authorizationStateWaitCode",
                    "code_info": {
                        "@type": "authenticationCodeInfo",
                        "phone_number": "+1******00",
                        "type": {"@type": "authenticationCodeTypeTelegramMessage", "length": 5},
                        "next_type": null,
                        "timeout": 60
                    }
                }),
                now,
            )
            .unwrap();
        assert!(!coordinator.is_identity_verified());
        assert_eq!(coordinator.account_kind(), None);
    }

    #[test]
    fn runtime_unavailable_is_not_sent_and_does_not_poison_the_challenge() {
        let now = Instant::now();
        let mut coordinator = AuthorizationCoordinator::with_epoch(11);
        coordinator
            .observe(&json!({"@type": "authorizationStateWaitPhoneNumber"}), now)
            .unwrap();
        let (state, token) = coordinator.status();
        assert_eq!(state, LoginState::PhoneNumber);
        let token = token.unwrap();

        for _ in 0..2 {
            assert_eq!(
                coordinator.submit(
                    None,
                    &token,
                    LoginInput::PhoneNumber {
                        value: ProtectedString::new("+10000000000".to_owned()),
                    },
                    now,
                ),
                DaemonResponse::CommandError {
                    code: CommandErrorCode::RuntimeUnavailable,
                }
            );
        }
    }

    #[test]
    fn coordinator_exposes_token_but_redacts_protected_input() {
        let mut coordinator = AuthorizationCoordinator::with_epoch(1);
        coordinator
            .observe(
                &json!({"@type": "authorizationStateWaitPhoneNumber"}),
                Instant::now(),
            )
            .unwrap();
        let (state, token) = coordinator.status();
        assert_eq!(state, LoginState::PhoneNumber);
        let token = token.unwrap();
        let canary = "AUTH_INPUT_CANARY";
        let (challenge, request) = coordinator
            .begin_submit(
                &token,
                LoginInput::PhoneNumber {
                    value: ProtectedString::new(canary.to_owned()),
                },
            )
            .unwrap();
        assert_eq!(request.request_type(), "setAuthenticationPhoneNumber");
        assert!(!format!("{request:?}").contains(canary));
        coordinator
            .record_outcome(challenge, SubmissionOutcome::NotSent)
            .unwrap();
    }

    #[test]
    fn tokens_are_boot_scoped_and_old_tokens_fail_closed() {
        let now = Instant::now();
        let mut first = AuthorizationCoordinator::with_epoch(1);
        first
            .observe(&json!({"@type": "authorizationStateWaitPhoneNumber"}), now)
            .unwrap();
        let first_token = first.status().1.unwrap();

        let mut second = AuthorizationCoordinator::with_epoch(2);
        second
            .observe(&json!({"@type": "authorizationStateWaitPhoneNumber"}), now)
            .unwrap();
        assert_ne!(first_token, second.status().1.unwrap());

        first
            .observe(&json!({"@type": "authorizationStateWaitPhoneNumber"}), now)
            .unwrap();
        assert!(matches!(
            first.prompt(&first_token),
            Err(AuthorizationError::StaleChallenge)
        ));
    }

    #[test]
    fn owner_prompt_enables_qr_without_exposing_link_to_status() {
        let now = Instant::now();
        let mut coordinator = AuthorizationCoordinator::with_epoch(7);
        coordinator
            .observe(&json!({"@type": "authorizationStateWaitPhoneNumber"}), now)
            .unwrap();
        let token = coordinator.status().1.unwrap();
        let (challenge, request) = coordinator
            .begin_submit(&token, LoginInput::QrCode)
            .unwrap();
        assert_eq!(request.request_type(), "requestQrCodeAuthentication");
        coordinator
            .record_outcome(challenge, SubmissionOutcome::NotSent)
            .unwrap();

        let canary = "tg://login?token=OWNER_PROMPT_CANARY";
        coordinator
            .observe(
                &json!({
                    "@type": "authorizationStateWaitOtherDeviceConfirmation",
                    "link": canary
                }),
                now,
            )
            .unwrap();
        let token = coordinator.status().1.unwrap();
        let prompt = coordinator.prompt(&token).unwrap();
        assert!(!format!("{prompt:?}").contains(canary));
        assert_eq!(coordinator.status().0, LoginState::QrCode);
    }

    #[test]
    fn uncertain_submission_reconciles_late_rejection() {
        let now = Instant::now();
        let mut coordinator = AuthorizationCoordinator::with_epoch(9);
        coordinator
            .observe(&json!({"@type": "authorizationStateWaitPhoneNumber"}), now)
            .unwrap();
        let token = coordinator.status().1.unwrap();
        let input = || LoginInput::PhoneNumber {
            value: ProtectedString::new("+10000000000".to_owned()),
        };
        let (challenge, _) = coordinator.begin_submit(&token, input()).unwrap();
        coordinator
            .record_outcome(challenge, SubmissionOutcome::Uncertain)
            .unwrap();
        coordinator.uncertain_submission = Some((42, challenge));
        assert!(matches!(
            coordinator.begin_submit(&token, input()),
            Err(AuthorizationError::SubmissionPending)
        ));

        coordinator
            .reconcile_response(42, &json!({"@type": "error", "code": 500}))
            .unwrap();
        assert!(coordinator.begin_submit(&token, input()).is_ok());
    }

    #[test]
    fn resend_uses_tdlib_deadline_and_supports_email_code() {
        let observed_at = Instant::now();
        let mut coordinator = AuthorizationCoordinator::with_epoch(13);
        coordinator
            .observe(
                &json!({
                    "@type": "authorizationStateWaitCode",
                    "code_info": {
                        "@type": "authenticationCodeInfo",
                        "phone_number": "+1******00",
                        "type": {"@type": "authenticationCodeTypeTelegramMessage", "length": 5},
                        "next_type": {"@type": "authenticationCodeTypeSms", "length": 5},
                        "timeout": 60
                    }
                }),
                observed_at,
            )
            .unwrap();
        let token = coordinator.status().1.unwrap();
        assert!(matches!(
            coordinator.begin_resend(&token, observed_at + Duration::from_secs(59)),
            Err(AuthorizationError::CodeResendUnavailable)
        ));
        let (challenge, request) = coordinator
            .begin_resend(&token, observed_at + Duration::from_secs(60))
            .unwrap();
        assert_eq!(request.request_type(), "resendAuthenticationCode");
        coordinator
            .record_outcome(challenge, SubmissionOutcome::NotSent)
            .unwrap();

        coordinator
            .observe(
                &json!({
                    "@type": "authorizationStateWaitEmailCode",
                    "allow_apple_id": false,
                    "allow_google_id": false,
                    "code_info": {
                        "@type": "emailAddressAuthenticationCodeInfo",
                        "email_address_pattern": "o***@example.test",
                        "length": 6
                    },
                    "email_address_reset_state": null
                }),
                observed_at,
            )
            .unwrap();
        let token = coordinator.status().1.unwrap();
        let (_, request) = coordinator.begin_resend(&token, observed_at).unwrap();
        assert_eq!(request.request_type(), "resendAuthenticationCode");
    }

    #[test]
    fn transient_tdlib_errors_are_not_classified_as_bad_otp() {
        assert_eq!(
            super::login_submission_error(&json!({"@type": "error", "code": 400})),
            CommandErrorCode::LoginSubmissionRejected
        );
        assert_eq!(
            super::login_submission_error(&json!({"@type": "error", "code": 429})),
            CommandErrorCode::TdlibTransport
        );
        assert_eq!(
            super::login_submission_error(&json!({"@type": "error", "code": 500})),
            CommandErrorCode::TdlibTransport
        );
    }
}
