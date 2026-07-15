use std::collections::BTreeSet;

use super::FeatureId;

#[test]
fn feature_ids_are_complete_unique_and_canonical() {
    assert_eq!(FeatureId::ALL.len(), 22);

    let values: Vec<_> = FeatureId::ALL.iter().map(|id| id.as_str()).collect();
    assert_eq!(values.first(), Some(&"F001"));
    assert_eq!(values.last(), Some(&"F022"));
    assert_eq!(values.iter().copied().collect::<BTreeSet<_>>().len(), 22);

    for (expected, value) in FeatureId::ALL.into_iter().zip(values) {
        assert_eq!(FeatureId::try_from(value), Ok(expected));
        assert_eq!(value.parse::<FeatureId>(), Ok(expected));
    }
}

#[test]
fn parser_rejects_values_outside_the_product_inventory() {
    for value in ["", "F000", "F023", "f001", "F01", " F001"] {
        assert!(FeatureId::try_from(value).is_err(), "accepted {value:?}");
    }
}
