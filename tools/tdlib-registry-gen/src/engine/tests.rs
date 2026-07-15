use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fmt::Write;

use super::{
    GenerationErrorKind, MAX_MANIFEST_BYTES, MAX_POLICY_BYTES, MAX_SCHEMA_BYTES, generate,
};

const SCHEMA: &str = "\
string ? = String;\n\
int53 = Int53;\n\
ok = Ok;\n\
user = User;\n\
chat = Chat;\n\
---functions---\n\
getUser id:int53 = User;\n\
setUserName name:string = Ok;\n\
getChat id:int53 = Chat;\n\
deleteChat id:int53 = Ok;\n";

const USER_SET_SHA256: &str = "4fcc9bd48d78cd718fa78cb8c9eee7d5d12ab1e330f5abeb49e4ac2f612d19b5";
const CHAT_SET_SHA256: &str = "0ca418747e93361fab38c2c4f0e1019a393c90ea60a30c4ce1122ace44b3fbed";
const GET_CHAT_SET_SHA256: &str =
    "ae6a20fb175a392dc97aaa22b943e576af9cca999459c4f0aa6f3380c0ee4975";
const GET_CHAT_SIGNATURE_SHA256: &str =
    "c2e3367022fdd83810520015b763935423214581d0aea7f0eecff7f92fd9d708";

#[test]
fn canonical_output_does_not_depend_on_rule_or_atom_order() {
    let first = fixture_policy(false);
    let second = fixture_policy(true);

    let first_output =
        generate(&fixture_manifest(), SCHEMA.as_bytes(), &first).expect("complete owner manifest");
    let second_output =
        generate(&fixture_manifest(), SCHEMA.as_bytes(), &second).expect("same semantic policy");

    assert_eq!(first_output, second_output);
    assert_eq!(first_output.last(), Some(&b'\n'));

    let manifest: Value = serde_json::from_slice(&first_output).expect("canonical JSON");
    assert_eq!(manifest["counts"]["schema_methods"], 4);
    assert_eq!(manifest["counts"]["owned_methods"], 4);
    assert_eq!(manifest["counts"]["overrides"], 0);
    assert_eq!(manifest["methods"][0]["method"], "deleteChat");
    assert_eq!(manifest["methods"][0]["feature_id"], "F008");
    assert_eq!(manifest["methods"][3]["method"], "setUserName");
    assert_eq!(manifest["methods"][3]["feature_id"], "F007");
}

#[test]
fn rejects_rule_drift_even_when_the_expected_count_is_unchanged() {
    let mut policy: Value = serde_json::from_slice(&fixture_policy(false)).expect("policy");
    policy["rules"][0]["expected"]["method_set_sha256"] = Value::String("0".repeat(64));

    let error = generate(
        &fixture_manifest(),
        SCHEMA.as_bytes(),
        &serde_json::to_vec(&policy).expect("policy bytes"),
    )
    .expect_err("reviewed method set changed");

    assert_eq!(error.kind(), GenerationErrorKind::RuleDrift);
}

#[test]
fn exact_override_can_only_resolve_the_pinned_candidate_intersection() {
    let policy = fixture_policy_with_chat_overlap(json!({
        "method": "getChat",
        "owner": "F009",
        "expected_candidates": ["F008", "F009"],
        "signature_sha256": GET_CHAT_SIGNATURE_SHA256,
        "rationale": "Boundary fixture selects a reviewed candidate."
    }));
    let output =
        generate(&fixture_manifest(), SCHEMA.as_bytes(), &policy).expect("reviewed overlap");
    let manifest: Value = serde_json::from_slice(&output).expect("manifest");
    let get_chat = manifest["methods"]
        .as_array()
        .expect("method rows")
        .iter()
        .find(|row| row["method"] == "getChat")
        .expect("getChat row");
    assert_eq!(get_chat["feature_id"], "F009");
    assert_eq!(get_chat["owner_source"], "override");

    let stale = fixture_policy_with_chat_overlap(json!({
        "method": "getChat",
        "owner": "F009",
        "expected_candidates": ["F009"],
        "signature_sha256": GET_CHAT_SIGNATURE_SHA256,
        "rationale": "Candidate set is intentionally stale."
    }));
    let error =
        generate(&fixture_manifest(), SCHEMA.as_bytes(), &stale).expect_err("stale candidate set");
    assert_eq!(error.kind(), GenerationErrorKind::StaleOverride);
}

#[test]
fn unknown_or_ambiguous_methods_never_publish_a_partial_manifest() {
    let mut unowned: Value = serde_json::from_slice(&fixture_policy(false)).expect("policy");
    unowned["rules"].as_array_mut().expect("rules").pop();
    let error = generate(
        &fixture_manifest(),
        SCHEMA.as_bytes(),
        &serde_json::to_vec(&unowned).expect("policy bytes"),
    )
    .expect_err("chat methods are unowned");
    assert_eq!(error.kind(), GenerationErrorKind::Coverage);

    let ambiguous = fixture_policy_with_chat_overlap(json!(null));
    let error = generate(&fixture_manifest(), SCHEMA.as_bytes(), &ambiguous)
        .expect_err("getChat has two candidates");
    assert_eq!(error.kind(), GenerationErrorKind::Coverage);
}

#[test]
fn rejects_generic_fallback_and_duplicate_feature_rules() {
    let mut fallback: Value = serde_json::from_slice(&fixture_policy(false)).expect("policy");
    fallback["rules"][1]["feature_id"] = json!("F020");
    fallback["rules"][1]["any"] = json!([{ "kind": "prefix", "value": "get" }]);
    let error = generate(
        &fixture_manifest(),
        SCHEMA.as_bytes(),
        &serde_json::to_vec(&fallback).expect("policy bytes"),
    )
    .expect_err("generic fallback");
    assert_eq!(error.kind(), GenerationErrorKind::InvalidPolicy);

    let mut duplicate: Value = serde_json::from_slice(&fixture_policy(false)).expect("policy");
    duplicate["rules"][1]["feature_id"] = json!("F007");
    let error = generate(
        &fixture_manifest(),
        SCHEMA.as_bytes(),
        &serde_json::to_vec(&duplicate).expect("policy bytes"),
    )
    .expect_err("duplicate feature rule");
    assert_eq!(error.kind(), GenerationErrorKind::InvalidPolicy);
}

#[test]
fn rejects_unknown_fields_features_and_dead_atoms() {
    let mut unknown_field: Value = serde_json::from_slice(&fixture_policy(false)).expect("policy");
    unknown_field["priority"] = json!(1);
    assert_error_kind(&unknown_field, GenerationErrorKind::InvalidPolicy);

    let mut unknown_feature: Value =
        serde_json::from_slice(&fixture_policy(false)).expect("policy");
    unknown_feature["rules"][0]["feature_id"] = json!("F023");
    assert_error_kind(&unknown_feature, GenerationErrorKind::InvalidPolicy);

    let mut dead_atom: Value = serde_json::from_slice(&fixture_policy(false)).expect("policy");
    dead_atom["rules"][0]["any"]
        .as_array_mut()
        .expect("atoms")
        .push(json!({ "kind": "exact", "value": "missingMethod" }));
    assert_error_kind(&dead_atom, GenerationErrorKind::InvalidPolicy);

    let mut duplicate_atom: Value = serde_json::from_slice(&fixture_policy(false)).expect("policy");
    let atom = duplicate_atom["rules"][0]["any"][0].clone();
    duplicate_atom["rules"][0]["any"]
        .as_array_mut()
        .expect("atoms")
        .push(atom);
    assert_error_kind(&duplicate_atom, GenerationErrorKind::InvalidPolicy);
}

#[test]
fn rejects_stale_schema_signature_and_examples() {
    let mut schema_drift: Value = serde_json::from_slice(&fixture_policy(false)).expect("policy");
    schema_drift["schema_sha256"] = Value::String("0".repeat(64));
    assert_error_kind(&schema_drift, GenerationErrorKind::SchemaDrift);

    let mut stale_signature: Value =
        serde_json::from_slice(&fixture_policy_with_chat_overlap(json!({
            "method": "getChat",
            "owner": "F009",
            "expected_candidates": ["F008", "F009"],
            "signature_sha256": "0".repeat(64),
            "rationale": "The signature evidence is intentionally stale."
        })))
        .expect("policy");
    assert_error_kind(&stale_signature, GenerationErrorKind::StaleOverride);

    let mut wrong_positive: Value = serde_json::from_slice(&fixture_policy(false)).expect("policy");
    wrong_positive["rules"][0]["positive_examples"] = json!(["getChat"]);
    assert_error_kind(&wrong_positive, GenerationErrorKind::InvalidPolicy);

    stale_signature["overrides"][0]["owner"] = json!("F010");
    stale_signature["overrides"][0]["signature_sha256"] = json!(GET_CHAT_SIGNATURE_SHA256);
    assert_error_kind(&stale_signature, GenerationErrorKind::InvalidPolicy);
}

#[test]
fn rejects_redundant_override_and_canonicalizes_candidate_order() {
    let redundant = policy(
        serde_json::from_slice::<Value>(&fixture_policy(false)).expect("policy")["rules"]
            .as_array()
            .expect("rules")
            .clone(),
        vec![json!({
            "method": "getUser",
            "owner": "F007",
            "expected_candidates": ["F007"],
            "signature_sha256": sha256_hex(b"getUser id:int53 = User;"),
            "rationale": "A redundant override must never become an implicit exception."
        })],
    );
    let error = generate(&fixture_manifest(), SCHEMA.as_bytes(), &redundant)
        .expect_err("single-candidate override is stale");
    assert_eq!(error.kind(), GenerationErrorKind::StaleOverride);

    let first = fixture_policy_with_chat_overlap(json!({
        "method": "getChat",
        "owner": "F009",
        "expected_candidates": ["F008", "F009"],
        "signature_sha256": GET_CHAT_SIGNATURE_SHA256,
        "rationale": "Boundary fixture selects a reviewed candidate."
    }));
    let second = fixture_policy_with_chat_overlap(json!({
        "method": "getChat",
        "owner": "F009",
        "expected_candidates": ["F009", "F008"],
        "signature_sha256": GET_CHAT_SIGNATURE_SHA256,
        "rationale": "Boundary fixture selects a reviewed candidate."
    }));
    assert_eq!(
        generate(&fixture_manifest(), SCHEMA.as_bytes(), &first).expect("first policy"),
        generate(&fixture_manifest(), SCHEMA.as_bytes(), &second).expect("second policy")
    );
}

#[test]
fn rejects_oversized_inputs_before_parsing() {
    for (manifest, schema, policy) in [
        (
            vec![b' '; MAX_MANIFEST_BYTES + 1],
            SCHEMA.as_bytes().to_vec(),
            fixture_policy(false),
        ),
        (
            fixture_manifest(),
            vec![b' '; MAX_SCHEMA_BYTES + 1],
            fixture_policy(false),
        ),
        (
            fixture_manifest(),
            SCHEMA.as_bytes().to_vec(),
            vec![b' '; MAX_POLICY_BYTES + 1],
        ),
    ] {
        let error = generate(&manifest, &schema, &policy).expect_err("input cap");
        assert_eq!(error.kind(), GenerationErrorKind::ResourceLimit);
    }
}

fn fixture_manifest() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "format_version": 1,
        "upstream": {
            "repository": "https://example.invalid/tdlib",
            "commit": "0123456789abcdef0123456789abcdef01234567",
            "version": "test"
        },
        "schema": {
            "sha256": sha256_hex(SCHEMA.as_bytes()),
            "definitions": 5,
            "functions": 4,
            "updates": 0,
            "authorization_states": 0
        }
    }))
    .expect("manifest")
}

fn fixture_policy(reverse: bool) -> Vec<u8> {
    let mut rules = vec![
        feature_rule(
            "F007",
            vec![
                json!({ "kind": "exact", "value": "setUserName" }),
                json!({ "kind": "contains", "value": "User" }),
            ],
            2,
            USER_SET_SHA256,
            "getUser",
            "getChat",
        ),
        feature_rule(
            "F008",
            vec![json!({ "kind": "contains", "value": "Chat" })],
            2,
            CHAT_SET_SHA256,
            "getChat",
            "getUser",
        ),
    ];
    if reverse {
        rules.reverse();
        rules[1]["any"].as_array_mut().expect("atoms").reverse();
    }
    policy(rules, Vec::new())
}

fn fixture_policy_with_chat_overlap(override_value: Value) -> Vec<u8> {
    let base: Value = serde_json::from_slice(&fixture_policy(false)).expect("policy");
    let mut rules = base["rules"].as_array().expect("rules").clone();
    rules.push(feature_rule(
        "F009",
        vec![json!({ "kind": "exact", "value": "getChat" })],
        1,
        GET_CHAT_SET_SHA256,
        "getChat",
        "deleteChat",
    ));
    let overrides = if override_value.is_null() {
        Vec::new()
    } else {
        vec![override_value]
    };
    policy(rules, overrides)
}

fn feature_rule(
    feature_id: &str,
    any: Vec<Value>,
    method_count: usize,
    method_set_sha256: &str,
    positive: &str,
    negative: &str,
) -> Value {
    json!({
        "feature_id": feature_id,
        "any": any,
        "expected": {
            "method_count": method_count,
            "method_set_sha256": method_set_sha256
        },
        "positive_examples": [positive],
        "negative_examples": [negative],
        "rationale": "Fixture rule with explicit positive and negative evidence."
    })
}

fn policy(rules: Vec<Value>, overrides: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "format_version": 1,
        "schema_sha256": sha256_hex(SCHEMA.as_bytes()),
        "rules": rules,
        "overrides": overrides
    }))
    .expect("policy")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

fn assert_error_kind(policy: &Value, expected: GenerationErrorKind) {
    let bytes = serde_json::to_vec(policy).expect("policy bytes");
    let error = generate(&fixture_manifest(), SCHEMA.as_bytes(), &bytes).expect_err("must fail");
    assert_eq!(error.kind(), expected);
}
