//! Exhaustive authorization-state/challenge machine для pinned TDLib.

use std::fmt;

use serde_json::{Map, Value, json};
use zeroize::Zeroizing;

#[derive(Clone, PartialEq, Eq)]
pub struct SensitiveString(Zeroizing<String>);

impl SensitiveString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(Zeroizing::new(value.into()))
    }

    pub fn expose_secret(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Debug for SensitiveString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeInfo {
    pub phone_number: SensitiveString,
    pub delivery_type: String,
    pub next_delivery_type: Option<String>,
    pub timeout_seconds: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailCodeInfo {
    pub email_address_pattern: SensitiveString,
    pub length: i32,
    pub reset_state: Option<EmailAddressResetState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmailAddressResetState {
    Available { wait_period_seconds: i32 },
    Pending { reset_in_seconds: i32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationTerms {
    pub text: String,
    pub minimum_user_age: i32,
    pub show_popup: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationState {
    WaitTdlibParameters,
    WaitPhoneNumber,
    WaitPremiumPurchase {
        store_product_id: String,
        premium_day_count: i32,
        support_email_address: SensitiveString,
        support_email_subject: String,
    },
    WaitEmailAddress {
        allow_apple_id: bool,
        allow_google_id: bool,
    },
    WaitEmailCode {
        allow_apple_id: bool,
        allow_google_id: bool,
        code: EmailCodeInfo,
    },
    WaitCode(CodeInfo),
    WaitOtherDeviceConfirmation {
        link: SensitiveString,
    },
    WaitRegistration(RegistrationTerms),
    WaitPassword {
        password_hint: SensitiveString,
        has_recovery_email_address: bool,
        has_passport_data: bool,
        recovery_email_address_pattern: SensitiveString,
    },
    Ready,
    LoggingOut,
    Closing,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChallengeId(u64);

impl ChallengeId {
    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationChallenge {
    pub id: ChallengeId,
    pub kind: AuthorizationChallengeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationChallengeKind {
    PhoneNumber,
    PremiumPurchase {
        store_product_id: String,
        premium_day_count: i32,
        support_email_address: SensitiveString,
        support_email_subject: String,
    },
    EmailAddress {
        allow_apple_id: bool,
        allow_google_id: bool,
    },
    EmailCode {
        allow_apple_id: bool,
        allow_google_id: bool,
        info: EmailCodeInfo,
    },
    AuthenticationCode(CodeInfo),
    OtherDeviceConfirmation {
        link: SensitiveString,
    },
    Registration(RegistrationTerms),
    Password {
        password_hint: SensitiveString,
        has_recovery_email_address: bool,
        has_passport_data: bool,
        recovery_email_address_pattern: SensitiveString,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationStep {
    ParametersRequired { generation: ChallengeId },
    Challenge(AuthorizationChallenge),
    Ready,
    LoggingOut,
    Closing,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationInput {
    PhoneNumber(SensitiveString),
    QrCode {
        other_user_ids: Vec<i64>,
    },
    EmailAddress(SensitiveString),
    EmailCode(SensitiveString),
    AppleIdToken(SensitiveString),
    GoogleIdToken(SensitiveString),
    AuthenticationCode(SensitiveString),
    Password(SensitiveString),
    Registration {
        first_name: SensitiveString,
        last_name: SensitiveString,
        terms_accepted: bool,
        disable_notification: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmissionOutcome {
    NotSent,
    DefinitiveRejected,
    Uncertain,
}

pub struct AuthorizationRequest(Value);

impl AuthorizationRequest {
    pub(crate) fn new(value: Value) -> Self {
        Self(value)
    }

    pub fn request_type(&self) -> &str {
        self.0["@type"].as_str().unwrap_or("invalid")
    }

    pub fn into_value(self) -> Value {
        self.0
    }
}

impl fmt::Debug for AuthorizationRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizationRequest")
            .field("request_type", &self.request_type())
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationError {
    ExpectedAuthorizationUpdate,
    ExpectedObject(&'static str),
    MissingField(&'static str),
    InvalidField(&'static str),
    UnknownAuthorizationState(String),
    ChallengeGenerationExhausted,
    NoCurrentChallenge,
    StaleChallenge,
    SubmissionPending,
    CodeResendUnavailable,
    InputDoesNotMatchState,
    DatabaseKeyRejected,
}

impl fmt::Display for AuthorizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpectedAuthorizationUpdate => {
                formatter.write_str("expected updateAuthorizationState")
            }
            Self::ExpectedObject(field) => write!(formatter, "{field} must be an object"),
            Self::MissingField(field) => write!(formatter, "missing authorization field {field}"),
            Self::InvalidField(field) => write!(formatter, "invalid authorization field {field}"),
            Self::UnknownAuthorizationState(state) => {
                write!(formatter, "unknown authorization state {state}")
            }
            Self::ChallengeGenerationExhausted => {
                formatter.write_str("authorization challenge generation exhausted")
            }
            Self::NoCurrentChallenge => formatter.write_str("no current authorization challenge"),
            Self::StaleChallenge => formatter.write_str("authorization challenge is stale"),
            Self::SubmissionPending => {
                formatter.write_str("authorization submission is already pending")
            }
            Self::CodeResendUnavailable => {
                formatter.write_str("authentication code can't be resent yet")
            }
            Self::InputDoesNotMatchState => {
                formatter.write_str("authorization input does not match current state")
            }
            Self::DatabaseKeyRejected => {
                formatter.write_str("database key was rejected; authorization is fail-closed")
            }
        }
    }
}

impl std::error::Error for AuthorizationError {}

#[derive(Debug, Default)]
pub struct AuthorizationMachine {
    current: Option<AuthorizationState>,
    generation: u64,
    submission_pending: bool,
    database_key_rejected: bool,
}

impl AuthorizationMachine {
    pub fn current_state(&self) -> Option<&AuthorizationState> {
        self.current.as_ref()
    }

    pub fn observe_update(
        &mut self,
        update: &Value,
    ) -> Result<AuthorizationStep, AuthorizationError> {
        let object = object(update, "authorization update")?;
        if string(object, "@type")? != "updateAuthorizationState" {
            return Err(AuthorizationError::ExpectedAuthorizationUpdate);
        }
        self.observe_state(
            object
                .get("authorization_state")
                .ok_or(AuthorizationError::MissingField("authorization_state"))?,
        )
    }

    pub fn observe_state(
        &mut self,
        state: &Value,
    ) -> Result<AuthorizationStep, AuthorizationError> {
        let state = parse_authorization_state(state)?;
        if self.database_key_rejected && !matches!(state, AuthorizationState::WaitTdlibParameters) {
            return Err(AuthorizationError::DatabaseKeyRejected);
        }
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(AuthorizationError::ChallengeGenerationExhausted)?;
        self.submission_pending = false;
        let step = step_for(&state, ChallengeId(self.generation));
        self.current = Some(state);
        Ok(step)
    }

    pub fn submit(
        &mut self,
        challenge: ChallengeId,
        input: AuthorizationInput,
    ) -> Result<AuthorizationRequest, AuthorizationError> {
        let state = self
            .current
            .as_ref()
            .ok_or(AuthorizationError::NoCurrentChallenge)?;
        if challenge != ChallengeId(self.generation) {
            return Err(AuthorizationError::StaleChallenge);
        }
        if self.submission_pending {
            return Err(AuthorizationError::SubmissionPending);
        }
        let request = request_for(state, input)?;
        self.submission_pending = true;
        Ok(AuthorizationRequest(request))
    }

    #[cfg(unix)]
    pub fn submit_parameters(
        &mut self,
        generation: ChallengeId,
        parameters: crate::database_key::TdlibParameters,
        key: &crate::database_key::DatabaseKey,
    ) -> Result<AuthorizationRequest, AuthorizationError> {
        if generation != ChallengeId(self.generation) {
            return Err(AuthorizationError::StaleChallenge);
        }
        if !matches!(self.current, Some(AuthorizationState::WaitTdlibParameters)) {
            return Err(AuthorizationError::InputDoesNotMatchState);
        }
        if self.submission_pending {
            return Err(AuthorizationError::SubmissionPending);
        }
        let request = parameters.into_request(key)?;
        self.database_key_rejected = false;
        self.submission_pending = true;
        Ok(request)
    }

    pub fn parameters_failed(
        &mut self,
        generation: ChallengeId,
        tdlib_error_code: i32,
    ) -> Result<(), AuthorizationError> {
        if !matches!(self.current, Some(AuthorizationState::WaitTdlibParameters)) {
            return Err(AuthorizationError::NoCurrentChallenge);
        }
        if generation != ChallengeId(self.generation) {
            return Err(AuthorizationError::StaleChallenge);
        }
        self.submission_pending = false;
        if tdlib_error_code == 401 {
            self.database_key_rejected = true;
        }
        Ok(())
    }

    pub fn submission_outcome(
        &mut self,
        challenge: ChallengeId,
        outcome: SubmissionOutcome,
    ) -> Result<(), AuthorizationError> {
        if self.current.is_none() {
            return Err(AuthorizationError::NoCurrentChallenge);
        }
        if matches!(self.current, Some(AuthorizationState::WaitTdlibParameters)) {
            return Err(AuthorizationError::InputDoesNotMatchState);
        }
        if challenge != ChallengeId(self.generation) {
            return Err(AuthorizationError::StaleChallenge);
        }
        if outcome != SubmissionOutcome::Uncertain {
            self.submission_pending = false;
        }
        Ok(())
    }

    pub fn submission_failed(&mut self, challenge: ChallengeId) -> Result<(), AuthorizationError> {
        self.submission_outcome(challenge, SubmissionOutcome::DefinitiveRejected)
    }

    pub fn resend_code(
        &mut self,
        challenge: ChallengeId,
    ) -> Result<AuthorizationRequest, AuthorizationError> {
        let state = self
            .current
            .as_ref()
            .ok_or(AuthorizationError::NoCurrentChallenge)?;
        if challenge != ChallengeId(self.generation) {
            return Err(AuthorizationError::StaleChallenge);
        }
        if self.submission_pending {
            return Err(AuthorizationError::SubmissionPending);
        }
        match state {
            AuthorizationState::WaitCode(info) if info.next_delivery_type.is_some() => {}
            AuthorizationState::WaitCode(_) => {
                return Err(AuthorizationError::CodeResendUnavailable);
            }
            AuthorizationState::WaitEmailCode { .. } => {}
            _ => return Err(AuthorizationError::InputDoesNotMatchState),
        }
        self.submission_pending = true;
        Ok(AuthorizationRequest::new(json!({
            "@type": "resendAuthenticationCode",
            "reason": {"@type": "resendCodeReasonUserRequest"}
        })))
    }
}

pub fn parse_authorization_state(state: &Value) -> Result<AuthorizationState, AuthorizationError> {
    let state = object(state, "authorization_state")?;
    match string(state, "@type")? {
        "authorizationStateWaitTdlibParameters" => Ok(AuthorizationState::WaitTdlibParameters),
        "authorizationStateWaitPhoneNumber" => Ok(AuthorizationState::WaitPhoneNumber),
        "authorizationStateWaitPremiumPurchase" => Ok(AuthorizationState::WaitPremiumPurchase {
            store_product_id: string(state, "store_product_id")?.to_owned(),
            premium_day_count: integer(state, "premium_day_count")?,
            support_email_address: SensitiveString::new(string(state, "support_email_address")?),
            support_email_subject: string(state, "support_email_subject")?.to_owned(),
        }),
        "authorizationStateWaitEmailAddress" => Ok(AuthorizationState::WaitEmailAddress {
            allow_apple_id: boolean(state, "allow_apple_id")?,
            allow_google_id: boolean(state, "allow_google_id")?,
        }),
        "authorizationStateWaitEmailCode" => Ok(AuthorizationState::WaitEmailCode {
            allow_apple_id: boolean(state, "allow_apple_id")?,
            allow_google_id: boolean(state, "allow_google_id")?,
            code: parse_email_code_info(state)?,
        }),
        "authorizationStateWaitCode" => Ok(AuthorizationState::WaitCode(parse_code_info(
            state
                .get("code_info")
                .ok_or(AuthorizationError::MissingField("code_info"))?,
        )?)),
        "authorizationStateWaitOtherDeviceConfirmation" => {
            Ok(AuthorizationState::WaitOtherDeviceConfirmation {
                link: SensitiveString::new(string(state, "link")?),
            })
        }
        "authorizationStateWaitRegistration" => Ok(AuthorizationState::WaitRegistration(
            parse_registration_terms(
                state
                    .get("terms_of_service")
                    .ok_or(AuthorizationError::MissingField("terms_of_service"))?,
            )?,
        )),
        "authorizationStateWaitPassword" => Ok(AuthorizationState::WaitPassword {
            password_hint: SensitiveString::new(string(state, "password_hint")?),
            has_recovery_email_address: boolean(state, "has_recovery_email_address")?,
            has_passport_data: boolean(state, "has_passport_data")?,
            recovery_email_address_pattern: SensitiveString::new(string(
                state,
                "recovery_email_address_pattern",
            )?),
        }),
        "authorizationStateReady" => Ok(AuthorizationState::Ready),
        "authorizationStateLoggingOut" => Ok(AuthorizationState::LoggingOut),
        "authorizationStateClosing" => Ok(AuthorizationState::Closing),
        "authorizationStateClosed" => Ok(AuthorizationState::Closed),
        unknown => Err(AuthorizationError::UnknownAuthorizationState(
            unknown.to_owned(),
        )),
    }
}

fn step_for(state: &AuthorizationState, id: ChallengeId) -> AuthorizationStep {
    let kind = match state {
        AuthorizationState::WaitTdlibParameters => {
            return AuthorizationStep::ParametersRequired { generation: id };
        }
        AuthorizationState::WaitPhoneNumber => AuthorizationChallengeKind::PhoneNumber,
        AuthorizationState::WaitPremiumPurchase {
            store_product_id,
            premium_day_count,
            support_email_address,
            support_email_subject,
        } => AuthorizationChallengeKind::PremiumPurchase {
            store_product_id: store_product_id.clone(),
            premium_day_count: *premium_day_count,
            support_email_address: support_email_address.clone(),
            support_email_subject: support_email_subject.clone(),
        },
        AuthorizationState::WaitEmailAddress {
            allow_apple_id,
            allow_google_id,
        } => AuthorizationChallengeKind::EmailAddress {
            allow_apple_id: *allow_apple_id,
            allow_google_id: *allow_google_id,
        },
        AuthorizationState::WaitEmailCode {
            allow_apple_id,
            allow_google_id,
            code,
        } => AuthorizationChallengeKind::EmailCode {
            allow_apple_id: *allow_apple_id,
            allow_google_id: *allow_google_id,
            info: code.clone(),
        },
        AuthorizationState::WaitCode(code) => {
            AuthorizationChallengeKind::AuthenticationCode(code.clone())
        }
        AuthorizationState::WaitOtherDeviceConfirmation { link } => {
            AuthorizationChallengeKind::OtherDeviceConfirmation { link: link.clone() }
        }
        AuthorizationState::WaitRegistration(terms) => {
            AuthorizationChallengeKind::Registration(terms.clone())
        }
        AuthorizationState::WaitPassword {
            password_hint,
            has_recovery_email_address,
            has_passport_data,
            recovery_email_address_pattern,
        } => AuthorizationChallengeKind::Password {
            password_hint: password_hint.clone(),
            has_recovery_email_address: *has_recovery_email_address,
            has_passport_data: *has_passport_data,
            recovery_email_address_pattern: recovery_email_address_pattern.clone(),
        },
        AuthorizationState::Ready => return AuthorizationStep::Ready,
        AuthorizationState::LoggingOut => return AuthorizationStep::LoggingOut,
        AuthorizationState::Closing => return AuthorizationStep::Closing,
        AuthorizationState::Closed => return AuthorizationStep::Closed,
    };
    AuthorizationStep::Challenge(AuthorizationChallenge { id, kind })
}

fn request_for(
    state: &AuthorizationState,
    input: AuthorizationInput,
) -> Result<Value, AuthorizationError> {
    match input {
        AuthorizationInput::PhoneNumber(phone) if can_restart_authentication(state) => {
            require_nonempty(&phone, "phone_number")?;
            Ok(json!({
                "@type": "setAuthenticationPhoneNumber",
                "phone_number": phone.expose_secret(),
                "settings": null
            }))
        }
        AuthorizationInput::QrCode { other_user_ids } if can_restart_authentication(state) => {
            if other_user_ids.iter().any(|id| *id <= 0) {
                return Err(AuthorizationError::InvalidField("other_user_ids"));
            }
            Ok(json!({
                "@type": "requestQrCodeAuthentication",
                "other_user_ids": other_user_ids
            }))
        }
        AuthorizationInput::EmailAddress(email)
            if matches!(state, AuthorizationState::WaitEmailAddress { .. }) =>
        {
            require_nonempty(&email, "email_address")?;
            Ok(json!({
                "@type": "setAuthenticationEmailAddress",
                "email_address": email.expose_secret()
            }))
        }
        AuthorizationInput::EmailCode(code)
            if matches!(state, AuthorizationState::WaitEmailCode { .. }) =>
        {
            require_nonempty(&code, "email_code")?;
            Ok(json!({
                "@type": "checkAuthenticationEmailCode",
                "code": {
                    "@type": "emailAddressAuthenticationCode",
                    "code": code.expose_secret()
                }
            }))
        }
        AuthorizationInput::AppleIdToken(token) if allows_apple_id(state) => {
            require_nonempty(&token, "apple_id_token")?;
            Ok(json!({
                "@type": "checkAuthenticationEmailCode",
                "code": {
                    "@type": "emailAddressAuthenticationAppleId",
                    "token": token.expose_secret()
                }
            }))
        }
        AuthorizationInput::GoogleIdToken(token) if allows_google_id(state) => {
            require_nonempty(&token, "google_id_token")?;
            Ok(json!({
                "@type": "checkAuthenticationEmailCode",
                "code": {
                    "@type": "emailAddressAuthenticationGoogleId",
                    "token": token.expose_secret()
                }
            }))
        }
        AuthorizationInput::AuthenticationCode(code)
            if matches!(state, AuthorizationState::WaitCode(_)) =>
        {
            require_nonempty(&code, "authentication_code")?;
            Ok(json!({
                "@type": "checkAuthenticationCode",
                "code": code.expose_secret()
            }))
        }
        AuthorizationInput::Password(password)
            if matches!(state, AuthorizationState::WaitPassword { .. }) =>
        {
            require_nonempty(&password, "password")?;
            Ok(json!({
                "@type": "checkAuthenticationPassword",
                "password": password.expose_secret()
            }))
        }
        AuthorizationInput::Registration {
            first_name,
            last_name,
            terms_accepted,
            disable_notification,
        } if matches!(state, AuthorizationState::WaitRegistration(_)) => {
            if !terms_accepted {
                return Err(AuthorizationError::InvalidField("terms_accepted"));
            }
            let first_length = first_name.expose_secret().chars().count();
            let last_length = last_name.expose_secret().chars().count();
            if !(1..=64).contains(&first_length) {
                return Err(AuthorizationError::InvalidField("first_name"));
            }
            if last_length > 64 {
                return Err(AuthorizationError::InvalidField("last_name"));
            }
            Ok(json!({
                "@type": "registerUser",
                "first_name": first_name.expose_secret(),
                "last_name": last_name.expose_secret(),
                "disable_notification": disable_notification
            }))
        }
        _ => Err(AuthorizationError::InputDoesNotMatchState),
    }
}

fn can_restart_authentication(state: &AuthorizationState) -> bool {
    matches!(
        state,
        AuthorizationState::WaitPhoneNumber
            | AuthorizationState::WaitPremiumPurchase { .. }
            | AuthorizationState::WaitEmailAddress { .. }
            | AuthorizationState::WaitEmailCode { .. }
            | AuthorizationState::WaitCode(_)
            | AuthorizationState::WaitRegistration(_)
            | AuthorizationState::WaitPassword { .. }
    )
}

fn allows_apple_id(state: &AuthorizationState) -> bool {
    matches!(
        state,
        AuthorizationState::WaitEmailAddress {
            allow_apple_id: true,
            ..
        } | AuthorizationState::WaitEmailCode {
            allow_apple_id: true,
            ..
        }
    )
}

fn allows_google_id(state: &AuthorizationState) -> bool {
    matches!(
        state,
        AuthorizationState::WaitEmailAddress {
            allow_google_id: true,
            ..
        } | AuthorizationState::WaitEmailCode {
            allow_google_id: true,
            ..
        }
    )
}

fn parse_code_info(value: &Value) -> Result<CodeInfo, AuthorizationError> {
    let info = object(value, "code_info")?;
    Ok(CodeInfo {
        phone_number: SensitiveString::new(string(info, "phone_number")?),
        delivery_type: nested_type(info, "type")?,
        next_delivery_type: optional_nested_type(info, "next_type")?,
        timeout_seconds: integer(info, "timeout")?,
    })
}

fn parse_email_code_info(state: &Map<String, Value>) -> Result<EmailCodeInfo, AuthorizationError> {
    let info = object(
        state
            .get("code_info")
            .ok_or(AuthorizationError::MissingField("code_info"))?,
        "code_info",
    )?;
    Ok(EmailCodeInfo {
        email_address_pattern: SensitiveString::new(string(info, "email_address_pattern")?),
        length: integer(info, "length")?,
        reset_state: parse_email_reset_state(state.get("email_address_reset_state"))?,
    })
}

fn parse_email_reset_state(
    value: Option<&Value>,
) -> Result<Option<EmailAddressResetState>, AuthorizationError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let reset = object(value, "email_address_reset_state")?;
    match string(reset, "@type")? {
        "emailAddressResetStateAvailable" => Ok(Some(EmailAddressResetState::Available {
            wait_period_seconds: integer(reset, "wait_period")?,
        })),
        "emailAddressResetStatePending" => Ok(Some(EmailAddressResetState::Pending {
            reset_in_seconds: integer(reset, "reset_in")?,
        })),
        _ => Err(AuthorizationError::InvalidField(
            "email_address_reset_state.@type",
        )),
    }
}

fn parse_registration_terms(value: &Value) -> Result<RegistrationTerms, AuthorizationError> {
    let terms = object(value, "terms_of_service")?;
    let formatted = object(
        terms
            .get("text")
            .ok_or(AuthorizationError::MissingField("text"))?,
        "terms_of_service.text",
    )?;
    Ok(RegistrationTerms {
        text: string(formatted, "text")?.to_owned(),
        minimum_user_age: integer(terms, "min_user_age")?,
        show_popup: boolean(terms, "show_popup")?,
    })
}

fn object<'a>(
    value: &'a Value,
    name: &'static str,
) -> Result<&'a Map<String, Value>, AuthorizationError> {
    value
        .as_object()
        .ok_or(AuthorizationError::ExpectedObject(name))
}

fn string<'a>(
    object: &'a Map<String, Value>,
    name: &'static str,
) -> Result<&'a str, AuthorizationError> {
    object
        .get(name)
        .ok_or(AuthorizationError::MissingField(name))?
        .as_str()
        .ok_or(AuthorizationError::InvalidField(name))
}

fn boolean(object: &Map<String, Value>, name: &'static str) -> Result<bool, AuthorizationError> {
    object
        .get(name)
        .ok_or(AuthorizationError::MissingField(name))?
        .as_bool()
        .ok_or(AuthorizationError::InvalidField(name))
}

fn integer(object: &Map<String, Value>, name: &'static str) -> Result<i32, AuthorizationError> {
    let value = object
        .get(name)
        .ok_or(AuthorizationError::MissingField(name))?
        .as_i64()
        .ok_or(AuthorizationError::InvalidField(name))?;
    i32::try_from(value).map_err(|_| AuthorizationError::InvalidField(name))
}

fn nested_type(
    object: &Map<String, Value>,
    name: &'static str,
) -> Result<String, AuthorizationError> {
    let nested = object
        .get(name)
        .ok_or(AuthorizationError::MissingField(name))?;
    Ok(string(self::object(nested, name)?, "@type")?.to_owned())
}

fn optional_nested_type(
    object: &Map<String, Value>,
    name: &'static str,
) -> Result<Option<String>, AuthorizationError> {
    match object.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => Ok(Some(
            string(self::object(value, name)?, "@type")?.to_owned(),
        )),
    }
}

fn require_nonempty(
    value: &SensitiveString,
    field: &'static str,
) -> Result<(), AuthorizationError> {
    if value.expose_secret().is_empty() {
        Err(AuthorizationError::InvalidField(field))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update(state: Value) -> Value {
        json!({
            "@type": "updateAuthorizationState",
            "authorization_state": state
        })
    }

    fn challenge_id(step: AuthorizationStep) -> ChallengeId {
        match step {
            AuthorizationStep::Challenge(challenge) => challenge.id,
            other => panic!("expected challenge, got {other:?}"),
        }
    }

    #[test]
    fn phone_qr_code_password_and_registration_requests_match_pinned_schema() {
        let mut machine = AuthorizationMachine::default();
        let phone_id = challenge_id(
            machine
                .observe_update(&update(json!({
                    "@type": "authorizationStateWaitPhoneNumber"
                })))
                .unwrap(),
        );
        let phone = machine
            .submit(
                phone_id,
                AuthorizationInput::PhoneNumber(SensitiveString::new("+10000000000")),
            )
            .unwrap();
        assert_eq!(phone.request_type(), "setAuthenticationPhoneNumber");
        assert_eq!(
            format!("{phone:?}"),
            "AuthorizationRequest { request_type: \"setAuthenticationPhoneNumber\", .. }"
        );
        assert_eq!(phone.into_value()["settings"], Value::Null);
        assert!(matches!(
            machine.submit(
                phone_id,
                AuthorizationInput::PhoneNumber(SensitiveString::new("+10000000000"))
            ),
            Err(AuthorizationError::SubmissionPending)
        ));
        machine.submission_failed(phone_id).unwrap();
        let qr = machine
            .submit(
                phone_id,
                AuthorizationInput::QrCode {
                    other_user_ids: vec![42],
                },
            )
            .unwrap()
            .into_value();
        assert_eq!(
            qr,
            json!({"@type": "requestQrCodeAuthentication", "other_user_ids": [42]})
        );

        let code_id = challenge_id(
            machine
                .observe_state(&json!({
                    "@type": "authorizationStateWaitCode",
                    "code_info": {
                        "@type": "authenticationCodeInfo",
                        "phone_number": "+1******00",
                        "type": {"@type": "authenticationCodeTypeSms", "length": 5},
                        "next_type": null,
                        "timeout": 60
                    }
                }))
                .unwrap(),
        );
        assert_eq!(
            machine
                .submit(
                    code_id,
                    AuthorizationInput::AuthenticationCode(SensitiveString::new("12345"))
                )
                .unwrap()
                .into_value(),
            json!({"@type": "checkAuthenticationCode", "code": "12345"})
        );

        let password_id = challenge_id(
            machine
                .observe_state(&json!({
                    "@type": "authorizationStateWaitPassword",
                    "password_hint": "hint",
                    "has_recovery_email_address": true,
                    "has_passport_data": false,
                    "recovery_email_address_pattern": "a***@example.test"
                }))
                .unwrap(),
        );
        assert_eq!(
            machine
                .submit(
                    password_id,
                    AuthorizationInput::Password(SensitiveString::new("secret"))
                )
                .unwrap()
                .into_value(),
            json!({"@type": "checkAuthenticationPassword", "password": "secret"})
        );

        let registration_id = challenge_id(
            machine
                .observe_state(&json!({
                    "@type": "authorizationStateWaitRegistration",
                    "terms_of_service": {
                        "@type": "termsOfService",
                        "text": {"@type": "formattedText", "text": "Terms", "entities": []},
                        "min_user_age": 18,
                        "show_popup": true
                    }
                }))
                .unwrap(),
        );
        assert_eq!(
            machine
                .submit(
                    registration_id,
                    AuthorizationInput::Registration {
                        first_name: SensitiveString::new("Ada"),
                        last_name: SensitiveString::new("Lovelace"),
                        terms_accepted: true,
                        disable_notification: true
                    }
                )
                .unwrap()
                .into_value(),
            json!({
                "@type": "registerUser",
                "first_name": "Ada",
                "last_name": "Lovelace",
                "disable_notification": true
            })
        );
    }

    #[test]
    fn code_resend_requires_next_delivery_type_and_builds_typed_request() {
        let mut machine = AuthorizationMachine::default();
        let unavailable_id = challenge_id(
            machine
                .observe_state(&json!({
                    "@type": "authorizationStateWaitCode",
                    "code_info": {
                        "@type": "authenticationCodeInfo",
                        "phone_number": "+1******00",
                        "type": {"@type": "authenticationCodeTypeTelegramMessage", "length": 5},
                        "next_type": null,
                        "timeout": 60
                    }
                }))
                .unwrap(),
        );
        assert!(matches!(
            machine.resend_code(unavailable_id),
            Err(AuthorizationError::CodeResendUnavailable)
        ));

        let resend_id = challenge_id(
            machine
                .observe_state(&json!({
                    "@type": "authorizationStateWaitCode",
                    "code_info": {
                        "@type": "authenticationCodeInfo",
                        "phone_number": "+1******00",
                        "type": {"@type": "authenticationCodeTypeTelegramMessage", "length": 5},
                        "next_type": {"@type": "authenticationCodeTypeSms", "length": 5},
                        "timeout": 60
                    }
                }))
                .unwrap(),
        );
        assert_eq!(
            machine.resend_code(resend_id).unwrap().into_value(),
            json!({
                "@type": "resendAuthenticationCode",
                "reason": {"@type": "resendCodeReasonUserRequest"}
            })
        );
    }

    #[test]
    fn email_code_resend_is_available_without_phone_timeout_metadata() {
        let mut machine = AuthorizationMachine::default();
        let challenge = challenge_id(
            machine
                .observe_state(&json!({
                    "@type": "authorizationStateWaitEmailCode",
                    "allow_apple_id": false,
                    "allow_google_id": false,
                    "code_info": {
                        "@type": "emailAddressAuthenticationCodeInfo",
                        "email_address_pattern": "o***@example.test",
                        "length": 6
                    },
                    "email_address_reset_state": null
                }))
                .unwrap(),
        );
        assert_eq!(
            machine.resend_code(challenge).unwrap().into_value(),
            json!({
                "@type": "resendAuthenticationCode",
                "reason": {"@type": "resendCodeReasonUserRequest"}
            })
        );
    }

    #[test]
    fn uncertain_submission_blocks_blind_replay_until_fresh_state() {
        let mut machine = AuthorizationMachine::default();
        let challenge = challenge_id(
            machine
                .observe_state(&json!({"@type": "authorizationStateWaitPhoneNumber"}))
                .unwrap(),
        );
        machine
            .submit(
                challenge,
                AuthorizationInput::PhoneNumber(SensitiveString::new("+10000000000")),
            )
            .unwrap();
        machine
            .submission_outcome(challenge, SubmissionOutcome::Uncertain)
            .unwrap();
        assert!(matches!(
            machine.submit(
                challenge,
                AuthorizationInput::PhoneNumber(SensitiveString::new("+10000000000"))
            ),
            Err(AuthorizationError::SubmissionPending)
        ));

        let next = challenge_id(
            machine
                .observe_state(&json!({"@type": "authorizationStateWaitPhoneNumber"}))
                .unwrap(),
        );
        assert_ne!(challenge, next);
        assert!(
            machine
                .submit(
                    next,
                    AuthorizationInput::PhoneNumber(SensitiveString::new("+10000000000"))
                )
                .is_ok()
        );
    }

    #[test]
    fn registration_decline_never_builds_register_user() {
        let mut machine = AuthorizationMachine::default();
        let challenge = challenge_id(
            machine
                .observe_state(&json!({
                    "@type": "authorizationStateWaitRegistration",
                    "terms_of_service": {
                        "@type": "termsOfService",
                        "text": {"@type": "formattedText", "text": "Terms", "entities": []},
                        "min_user_age": 18,
                        "show_popup": true
                    }
                }))
                .unwrap(),
        );
        assert!(matches!(
            machine.submit(
                challenge,
                AuthorizationInput::Registration {
                    first_name: SensitiveString::new("Ada"),
                    last_name: SensitiveString::new(""),
                    terms_accepted: false,
                    disable_notification: true,
                }
            ),
            Err(AuthorizationError::InvalidField("terms_accepted"))
        ));
    }

    #[test]
    fn email_and_device_branches_preserve_metadata_and_redact_secrets() {
        let mut machine = AuthorizationMachine::default();
        let address_id = challenge_id(
            machine
                .observe_state(&json!({
                    "@type": "authorizationStateWaitEmailAddress",
                    "allow_apple_id": true,
                    "allow_google_id": false
                }))
                .unwrap(),
        );
        assert_eq!(
            machine
                .submit(
                    address_id,
                    AuthorizationInput::EmailAddress(SensitiveString::new("owner@example.test"))
                )
                .unwrap()
                .into_value(),
            json!({"@type": "setAuthenticationEmailAddress", "email_address": "owner@example.test"})
        );
        machine.submission_failed(address_id).unwrap();
        assert_eq!(
            machine
                .submit(
                    address_id,
                    AuthorizationInput::AppleIdToken(SensitiveString::new("apple-token"))
                )
                .unwrap()
                .into_value(),
            json!({
                "@type": "checkAuthenticationEmailCode",
                "code": {"@type": "emailAddressAuthenticationAppleId", "token": "apple-token"}
            })
        );

        let email_state = json!({
        "@type": "authorizationStateWaitEmailCode",
        "allow_apple_id": true,
        "allow_google_id": false,
        "code_info": {
            "@type": "emailAddressAuthenticationCodeInfo",
            "email_address_pattern": "o***@example.test",
            "length": 6
        },
            "email_address_reset_state": {
                "@type": "emailAddressResetStateAvailable",
                "wait_period": 3600
            }
        });
        let email_step = machine.observe_state(&email_state).unwrap();
        let email_id = challenge_id(email_step.clone());
        assert!(!format!("{email_step:?}").contains("example.test"));
        assert!(matches!(
            email_step,
            AuthorizationStep::Challenge(AuthorizationChallenge {
                kind: AuthorizationChallengeKind::EmailCode {
                    info: EmailCodeInfo {
                        reset_state: Some(EmailAddressResetState::Available {
                            wait_period_seconds: 3600
                        }),
                        ..
                    },
                    ..
                },
                ..
            })
        ));
        assert_eq!(
            machine
                .submit(
                    email_id,
                    AuthorizationInput::EmailCode(SensitiveString::new("123456"))
                )
                .unwrap()
                .into_value(),
            json!({
                "@type": "checkAuthenticationEmailCode",
                "code": {"@type": "emailAddressAuthenticationCode", "code": "123456"}
            })
        );

        let device_step = machine
            .observe_state(&json!({
                "@type": "authorizationStateWaitOtherDeviceConfirmation",
                "link": "tg://login?token=sensitive"
            }))
            .unwrap();
        let device_id = challenge_id(device_step.clone());
        assert!(!format!("{device_step:?}").contains("sensitive"));
        let refreshed_id = challenge_id(
            machine
                .observe_state(&json!({
                    "@type": "authorizationStateWaitOtherDeviceConfirmation",
                    "link": "tg://login?token=refreshed"
                }))
                .unwrap(),
        );
        assert_ne!(device_id, refreshed_id);
        assert!(matches!(
            machine.submit(
                device_id,
                AuthorizationInput::QrCode {
                    other_user_ids: vec![]
                }
            ),
            Err(AuthorizationError::StaleChallenge)
        ));
    }

    #[test]
    fn parameters_premium_and_terminal_states_are_explicit_and_unknown_is_rejected() {
        let mut machine = AuthorizationMachine::default();
        assert!(matches!(
            machine
                .observe_state(&json!({"@type": "authorizationStateWaitTdlibParameters"}))
                .unwrap(),
            AuthorizationStep::ParametersRequired { .. }
        ));
        assert!(matches!(
            machine.submit(
                ChallengeId(1),
                AuthorizationInput::QrCode {
                    other_user_ids: vec![]
                }
            ),
            Err(AuthorizationError::InputDoesNotMatchState)
        ));

        let premium = machine
            .observe_state(&json!({
                "@type": "authorizationStateWaitPremiumPurchase",
                "store_product_id": "premium.product",
                "premium_day_count": 30,
                "support_email_address": "support@example.test",
                "support_email_subject": "Login"
            }))
            .unwrap();
        assert!(matches!(
            premium,
            AuthorizationStep::Challenge(AuthorizationChallenge {
                kind: AuthorizationChallengeKind::PremiumPurchase { .. },
                ..
            })
        ));
        assert!(!format!("{premium:?}").contains("support@example.test"));

        assert_eq!(
            machine
                .observe_state(&json!({"@type": "authorizationStateReady"}))
                .unwrap(),
            AuthorizationStep::Ready
        );
        assert_eq!(
            machine
                .observe_state(&json!({"@type": "authorizationStateLoggingOut"}))
                .unwrap(),
            AuthorizationStep::LoggingOut
        );
        assert_eq!(
            machine
                .observe_state(&json!({"@type": "authorizationStateClosing"}))
                .unwrap(),
            AuthorizationStep::Closing
        );
        assert_eq!(
            machine
                .observe_state(&json!({"@type": "authorizationStateClosed"}))
                .unwrap(),
            AuthorizationStep::Closed
        );
        assert_eq!(
            machine.observe_state(&json!({"@type": "authorizationStateFuture"})),
            Err(AuthorizationError::UnknownAuthorizationState(
                "authorizationStateFuture".into()
            ))
        );
    }
}
