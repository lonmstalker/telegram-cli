use super::*;
use crate::schema::Schema;
use serde_json::json;

#[test]
fn generated_registry_matches_parser_and_supports_lookup() {
    let schema = Schema::parse(include_str!("../../../../vendor/tdlib/td_api.tl")).unwrap();
    assert_eq!(METHODS.len(), schema.methods().len());
    assert_eq!(
        CONSTRUCTORS.len(),
        schema.inventory().constructor_names().len()
    );
    assert!(METHODS.windows(2).all(|pair| pair[0].name < pair[1].name));
    assert!(CONSTRUCTORS
        .windows(2)
        .all(|pair| pair[0].name < pair[1].name));
    assert!(UPDATES.iter().all(|name| {
        constructor(name).is_some_and(|descriptor| descriptor.result.name == "Update")
    }));
    assert_eq!(method("getMe").unwrap().result.name, "User");
    assert_eq!(constructor("user").unwrap().result.name, "User");
    assert_eq!(CAPABILITIES.len(), METHODS.len());
    assert!(CAPABILITIES
        .iter()
        .zip(METHODS)
        .all(|(capability, method)| capability.method == method.name));
}

#[test]
fn reviewed_capabilities_are_data_and_everything_else_is_default_deny() {
    assert!(matches!(
        capability("getChatStatistics").unwrap().disposition,
        CapabilityDisposition::Reviewed {
            risk: RiskClass::Read,
            accounts: &[AccountKind::RegularUser],
            retry: RetryClass::SafeRead,
            ..
        }
    ));
    assert_eq!(
        capability("getMe").unwrap().disposition,
        CapabilityDisposition::DefaultDeny
    );
}

#[test]
fn validates_known_requests_recursively() {
    let request = ValidatedRequest::from_value(json!({
        "@type": "setOption",
        "name": "ignore_sensitive_content_restrictions",
        "value": {"@type": "optionValueBoolean", "value": true}
    }))
    .unwrap();
    assert_eq!(request.descriptor().name, "setOption");

    ValidatedRequest::from_value(json!({
        "@type": "addProxy",
        "proxy": {"@type": "proxy"}
    }))
    .unwrap();

    assert_eq!(
        ValidatedRequest::from_value(json!({"@type": "getMe", "future": true})),
        Err(ValidationError::UnexpectedField { symbol: "getMe" })
    );
    assert_eq!(
        ValidatedRequest::from_value(json!({"@type": "futureMethod"})),
        Err(ValidationError::UnknownMethod)
    );
}

#[test]
fn every_constructor_round_trips_unknown_fields_losslessly() {
    for descriptor in CONSTRUCTORS {
        let value = json!({
            "@type": descriptor.name,
            "future_scalar": 9_007_199_254_740_991_i64,
            "future_nested": {"enabled": true, "items": [null, "value"]}
        });
        let object = TdObject::from_value(value.clone()).unwrap();
        assert_eq!(object.descriptor(), Some(descriptor));
        assert_eq!(object.into_value(), value);
    }

    let value = json!({"@type": "futureConstructor", "future": {"value": 1}});
    let object = TdObject::from_value(value.clone()).unwrap();
    assert_eq!(object.descriptor(), None);
    assert_eq!(object.into_value(), value);
}
