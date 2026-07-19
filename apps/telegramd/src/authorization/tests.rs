use std::time::{Duration, Instant};

use serde_json::json;
use telegram_core::authorization::{AuthorizationError, SubmissionOutcome};
use telegram_core::registry::AccountKind;
use telegram_protocol::{
    CommandErrorCode, DaemonResponse, LoginInput, LoginNextAction, LoginState, ProtectedString,
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
fn parameters_status_waits_without_a_wire_challenge_token() {
    let mut coordinator = AuthorizationCoordinator::with_epoch(1);
    coordinator
        .observe(
            &json!({"@type": "authorizationStateWaitTdlibParameters"}),
            Instant::now(),
        )
        .unwrap();

    assert_eq!(
        coordinator.status_response(),
        DaemonResponse::LoginStatus {
            state: LoginState::Parameters,
            challenge_id: None,
            next_action: LoginNextAction::Wait,
        }
    );
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
