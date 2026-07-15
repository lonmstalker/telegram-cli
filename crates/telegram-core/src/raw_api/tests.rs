use super::*;
use crate::registry::{AccountKind, CapabilityDisposition, RiskClass, SymbolKind};

#[test]
fn discovery_uses_generated_registry_without_method_wrappers() {
    assert_eq!(capabilities().len(), registry::METHODS.len());
    assert!(matches!(
        registry::capability("getMe").unwrap().disposition,
        CapabilityDisposition::DefaultDeny
    ));

    let results = schema_search("chat statistics");
    assert!(results
        .iter()
        .any(|result| result.name() == "getChatStatistics"));
    assert!(results
        .windows(2)
        .all(|pair| pair[0].name() <= pair[1].name()));

    assert!(matches!(
        schema_describe("getMe"),
        Some(SchemaDescription::Symbol(SymbolDescriptor {
            kind: SymbolKind::Method,
            name: "getMe",
            ..
        }))
    ));
    let Some(SchemaDescription::Type { constructors, .. }) = schema_describe("User") else {
        panic!("User type must be described");
    };
    assert!(constructors
        .iter()
        .any(|constructor| constructor.name == "user"));
}

#[test]
fn policy_is_default_deny_and_requires_account_and_risk() {
    let read = RawPolicy::new(AccountKind::RegularUser, vec![RiskClass::Read]);
    assert_eq!(read.authorize("getChatStatistics"), Ok(()));
    assert_eq!(read.authorize("getMe"), Err(PolicyError::DefaultDeny));
    assert_eq!(
        RawPolicy::new(AccountKind::RegularUser, vec![]).authorize("getChatStatistics"),
        Err(PolicyError::RiskDenied {
            risk: RiskClass::Read
        })
    );
    assert_eq!(
        RawPolicy::new(AccountKind::Bot, vec![RiskClass::Read]).authorize("getChatStatistics"),
        Err(PolicyError::AccountScopeDenied)
    );
}
