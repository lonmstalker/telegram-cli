use super::*;
use crate::registry::{CapabilityDisposition, SymbolKind};

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
