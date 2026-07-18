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
