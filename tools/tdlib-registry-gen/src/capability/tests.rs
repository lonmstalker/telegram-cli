use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use telegram_core::method_capability::{
    AccountKind, ApplicationRequirement, AuthorizationState, CapabilityDescriptor, DcEnvironment,
    RequirementAlternatives, SynchronousCapability,
};
use telegram_core::schema::Schema;

use super::{
    CapabilityGenerationErrorKind, MAX_CAPABILITY_POLICY_BYTES, MAX_MANIFEST_BYTES,
    MAX_OWNER_POLICY_BYTES, MAX_SCHEMA_BYTES, documentation_sha256,
    documented_runtime_requirements, field_type, generate, has_runtime_gate_signal,
    method_documentation_text, normalized_text, serialize_pretty_with_limit, sha256_hex,
    validate_documented_authorization_states, validate_documented_method_constraints,
    validate_documented_parameter_notices, validate_documented_runtime_requirements,
};

type PolicyMutation = Box<dyn Fn(&mut Value)>;

const SCHEMA: &str = r#"string ? = String;
int32 = Int32;
int53 = Int53;
boolFalse = Bool;
boolTrue = Bool;
ok = Ok;

authorizationStateWaitTdlibParameters = AuthorizationState;
authorizationStateWaitPhoneNumber = AuthorizationState;
authorizationStateWaitPremiumPurchase = AuthorizationState;
authorizationStateWaitEmailAddress = AuthorizationState;
authorizationStateWaitEmailCode = AuthorizationState;
authorizationStateWaitCode = AuthorizationState;
authorizationStateWaitOtherDeviceConfirmation = AuthorizationState;
authorizationStateWaitRegistration = AuthorizationState;
authorizationStateWaitPassword = AuthorizationState;
authorizationStateReady = AuthorizationState;
authorizationStateLoggingOut = AuthorizationState;
authorizationStateClosing = AuthorizationState;
authorizationStateClosed = AuthorizationState;

chatAdministratorRights is_anonymous:Bool can_manage_chat:Bool can_change_info:Bool can_post_messages:Bool can_edit_messages:Bool can_delete_messages:Bool can_invite_users:Bool can_restrict_members:Bool can_pin_messages:Bool can_manage_topics:Bool can_promote_members:Bool can_manage_video_chats:Bool can_post_stories:Bool can_edit_stories:Bool can_delete_stories:Bool can_manage_direct_messages:Bool can_manage_tags:Bool = ChatAdministratorRights;
chatPermissions can_send_basic_messages:Bool can_send_audios:Bool can_send_documents:Bool can_send_photos:Bool can_send_videos:Bool can_send_video_notes:Bool can_send_voice_notes:Bool can_send_polls:Bool can_send_other_messages:Bool can_add_link_previews:Bool can_react_to_messages:Bool can_edit_tag:Bool can_change_info:Bool can_invite_users:Bool can_pin_messages:Bool can_create_topics:Bool = ChatPermissions;
businessBotRights can_reply:Bool can_read_messages:Bool can_delete_sent_messages:Bool can_delete_all_messages:Bool can_edit_name:Bool can_edit_bio:Bool can_edit_profile_photo:Bool can_edit_username:Bool can_view_gifts_and_stars:Bool can_sell_gifts:Bool can_change_gift_settings:Bool can_transfer_and_upgrade_gifts:Bool can_transfer_stars:Bool can_manage_stories:Bool = BusinessBotRights;

---functions---

//@description Returns the current authorization state. Can be called before initialization
getAuthorizationState = Ok;

//@description Provides initialization parameters. Works only when the current authorization state is authorizationStateWaitTdlibParameters
setTdlibParameters = Ok;

//@description Returns a chat @chat_id Chat identifier
getChat chat_id:int53 = Ok;

//@description Uses a feature; for Telegram Premium users only
usePremiumFeature = Ok;

//@description Uses a feature. Requires Telegram Business subscription
useBusinessFeature = Ok;

//@description Submits an in-store payment; for regular users only; for official Telegram apps only
submitStorePayment = Ok;

//@description Sends a request in Test DC only
testNetworkRequest = Ok;

//@description Removes a pinned message from a synthetic chat; requires can_pin_messages member right or can_edit_messages administrator right @chat_id Chat identifier @message_id Message identifier @reason Diagnostic fixture
unpinChatMessage chat_id:int53 message_id:int53 reason:string = Ok;

//@description Toggles whether a topic is closed in a forum supergroup chat; requires can_manage_topics administrator right in the supergroup unless the user is creator of the topic @chat_id Chat identifier @forum_topic_id Forum topic identifier @other_topic_id Unrelated same-type identifier @is_closed New closed state
toggleForumTopicIsClosed chat_id:int53 forum_topic_id:int32 other_topic_id:int32 is_closed:Bool = Ok;

//@description Changes the sticker set of a supergroup; requires can_change_info administrator right @supergroup_id Identifier of the supergroup @other_supergroup_id Unrelated same-type identifier @sticker_set_id Sticker set identifier
setSupergroupStickerSet supergroup_id:int53 other_supergroup_id:int53 sticker_set_id:int53 = Ok;

//@description Requires synthetic administrator evidence in a supergroup @supergroup_id Identifier of the supergroup
requireSyntheticSupergroupAdministrator supergroup_id:int53 = Ok;

//@description Creates a new supergroup from an existing basic group and sends a corresponding messageChatUpgradeTo and messageChatUpgradeFrom; requires owner privileges. Deactivates the original basic group @chat_id Identifier of the chat to upgrade
upgradeBasicGroupChatToSupergroupChat chat_id:int53 = Ok;

//@description Sends on behalf of a business account; for bots only; requires an enabled business connection with can_reply right @business_connection_id Business connection identifier @reason Diagnostic fixture
sendBusinessMessage business_connection_id:string reason:string = Ok;

//@description Exercises value-level capability restrictions
//@bot_value Bot-provided data; bots only
//@premium_value Telegram Premium users can use additional values
//@business_value Some values require Telegram Business subscription
//@official_value Some values are available only to official Telegram apps
//@official_mobile_value Some values are available only to official mobile applications
//@production_value Some values are available only in the production environment
//@test_value Some values are available only if Telegram test environment is used
configureGatedValues bot_value:string premium_value:string business_value:string official_value:string official_mobile_value:string production_value:int32 test_value:int32 = Ok;

//@description Parses text. Can be called synchronously @text Text to parse
parseText text:string = Ok;

//@description Returns an option. Can be called before authorization. Can be called synchronously for options "version" and "commit_hash" @name Option name
getOption name:string = Ok;
"#;

#[test]
fn canonical_generation_is_pure_and_independent_of_policy_order() {
    let fixture = Fixture::new(SCHEMA);
    let first = fixture.generate().expect("complete capability manifest");
    let mut reordered = fixture.capability_value();
    reorder_policy(&mut reordered);
    let second = generate(
        &fixture.manifest,
        &fixture.schema,
        &fixture.owner_policy,
        &serde_json::to_vec(&reordered).expect("reordered policy"),
    )
    .expect("same semantic policy");

    assert_eq!(first, second);
    assert_eq!(first.last(), Some(&b'\n'));
    let artifact: Value = serde_json::from_slice(&first).expect("artifact JSON");
    assert_eq!(artifact["counts"]["schema_methods"], 16);
    assert_eq!(artifact["counts"]["capability_methods"], 16);
    let methods = artifact["methods"].as_array().expect("method rows");
    assert!(
        methods
            .windows(2)
            .all(|pair| pair[0]["method"].as_str() < pair[1]["method"].as_str())
    );

    let configure = method_row(&artifact, "configureGatedValues");
    assert_eq!(configure["parameter_notices"].as_array().unwrap().len(), 7);
    let unpin = method_row(&artifact, "unpinChatMessage");
    assert_eq!(unpin["runtime_requirements"]["kind"], "any_of");
    assert_eq!(
        method_row(&artifact, "setSupergroupStickerSet")["runtime_requirements"]["clauses"][0]["all_of"]
            [0]["target"]["kind"],
        "supergroup_id"
    );
    let get_option = method_row(&artifact, "getOption");
    assert_eq!(
        get_option["synchronous"]["values"],
        json!(["commit_hash", "version"])
    );
}

#[test]
fn requires_the_exact_pinned_authorization_state_inventory() {
    for schema in [
        SCHEMA.replace(
            "authorizationStateWaitEmailCode",
            "authorizationStateWaitMagic",
        ),
        SCHEMA.replace("authorizationStateWaitEmailCode = AuthorizationState;\n", ""),
        SCHEMA.replace(
            "authorizationStateWaitEmailCode = AuthorizationState;",
            "authorizationStateWaitEmailCode = AuthorizationState;\nauthorizationStateWaitMagic = AuthorizationState;",
        ),
    ] {
        let fixture = Fixture::new(&schema);
        assert_eq!(
            fixture.generate().expect_err("auth-state drift").kind(),
            CapabilityGenerationErrorKind::SchemaDrift
        );
    }
}

#[test]
fn requires_exact_schema_derived_right_vocabularies_at_generation_time() {
    for schema in [
        SCHEMA.replace("can_manage_tags:Bool", "can_manage_labels:Bool"),
        SCHEMA.replace("can_reply:Bool", "can_reply:int32"),
        SCHEMA.replace("chatPermissions ", "renamedChatPermissions "),
    ] {
        let fixture = Fixture::new(&schema);
        assert_eq!(
            fixture.generate().expect_err("rights drift").kind(),
            CapabilityGenerationErrorKind::SchemaDrift
        );
    }
}

#[test]
fn binds_method_rows_to_documented_authorization_contracts() {
    let fixture = Fixture::new(SCHEMA);
    for (method, states) in [
        ("setTdlibParameters", json!(["authorizationStateReady"])),
        (
            "getOption",
            json!([
                "authorizationStateWaitTdlibParameters",
                "authorizationStateReady"
            ]),
        ),
        ("getAuthorizationState", json!(["authorizationStateReady"])),
        (
            "getChat",
            json!([
                "authorizationStateReady",
                "authorizationStateWaitPhoneNumber"
            ]),
        ),
    ] {
        let mut policy = fixture.capability_value();
        let row = method_row_mut(&mut policy, method);
        row["authorization_states"] = states;
        if method == "setTdlibParameters" {
            row["ready_accounts"] = json!(["regular_user", "bot"]);
        }
        let error = fixture.generate_value(&policy).expect_err(method);
        assert_eq!(error.kind(), CapabilityGenerationErrorKind::InvalidPolicy);
        assert!(
            error
                .to_string()
                .contains("contradict method documentation")
        );
    }
}

#[test]
fn authorization_contract_reads_all_method_documentation_tags() {
    let schema =
        Schema::parse(include_str!("../../../../vendor/tdlib/td_api.tl")).expect("pinned schema");
    let method = find_method(&schema, "setCustomLanguagePack");
    let pre_authorization = CapabilityDescriptor::try_new(
        SynchronousCapability::Never,
        vec![AccountKind::RegularUser, AccountKind::Bot],
        requestable_authorization_states()
            .into_iter()
            .map(|state| AuthorizationState::try_from(state).expect("known state"))
            .collect(),
        Vec::new(),
        ApplicationRequirement::Any,
        vec![DcEnvironment::Production, DcEnvironment::Test],
        RequirementAlternatives::always(),
        Vec::new(),
    )
    .expect("pre-authorization descriptor");

    validate_documented_authorization_states(method, &pre_authorization)
        .expect("@info carries the method-level pre-authorization contract");
    let error = validate_documented_authorization_states(
        method,
        &ready_descriptor(ApplicationRequirement::Any),
    )
    .expect_err("Ready-only policy must not ignore a contract outside @description");
    assert_eq!(error.kind(), CapabilityGenerationErrorKind::InvalidPolicy);
}

#[test]
fn pinned_non_ready_authorization_contract_inventory_is_exact() {
    let schema =
        Schema::parse(include_str!("../../../../vendor/tdlib/td_api.tl")).expect("pinned schema");
    let ready_only = ready_descriptor(ApplicationRequirement::Any);
    let mut methods = schema
        .methods()
        .iter()
        .filter(|method| validate_documented_authorization_states(method, &ready_only).is_err())
        .map(|method| method.name())
        .collect::<Vec<_>>();
    methods.sort_unstable();
    let mut oracle = methods.join("\n");
    oracle.push('\n');
    assert_eq!(methods.len(), 73, "authorization-contract set drift");
    assert_eq!(
        sha256_hex(oracle.as_bytes()),
        "89a4dd651b3372d2310ddb6fa16e2e6827d0bd67b6555a8e5800694ceb0440b3",
        "authorization-contract method-set hash drift"
    );
}

#[test]
fn pinned_runtime_signal_inventory_and_open_disposition_boundary_are_exact() {
    let capability_source = include_str!("../capability.rs");
    let recognizer_start = capability_source
        .find("fn has_runtime_gate_signal")
        .expect("runtime recognizer start");
    let recognizer_end = capability_source[recognizer_start..]
        .find("fn contains_word_sequence")
        .map(|offset| recognizer_start + offset)
        .expect("runtime recognizer end");
    assert_eq!(
        sha256_hex(&capability_source.as_bytes()[recognizer_start..recognizer_end]),
        "5cdf338bec6fa08d0f69d31c999c8ca384581f96cb6105384cebf754e3e65f1a",
        "runtime recognizer body drift"
    );

    let schema =
        Schema::parse(include_str!("../../../../vendor/tdlib/td_api.tl")).expect("pinned schema");
    let signaled = schema
        .methods()
        .iter()
        .filter(|method| {
            has_runtime_gate_signal(&normalized_text(&method_documentation_text(method)))
        })
        .collect::<Vec<_>>();
    let mut names = signaled
        .iter()
        .map(|method| method.name())
        .collect::<Vec<_>>();
    names.sort_unstable();
    let mut oracle = names.join("\n");
    oracle.push('\n');
    assert_eq!(names.len(), 193, "runtime-signal method-set drift");
    assert_eq!(
        sha256_hex(oracle.as_bytes()),
        "cbe074623352b1b4e970af939aed6297e7ce37366d7fd5ad7cedcf1a36848706",
        "runtime-signal method-set hash drift"
    );

    let hash_method_set = |mut values: Vec<&str>| {
        values.sort_unstable();
        let mut payload = values.join("\n");
        payload.push('\n');
        (values.len(), sha256_hex(payload.as_bytes()))
    };
    let description_signals = schema
        .methods()
        .iter()
        .filter(|method| {
            method.documentation().tags().iter().any(|tag| {
                tag.name() == "description"
                    && has_runtime_gate_signal(&normalized_text(tag.value()))
            })
        })
        .map(|method| method.name())
        .collect::<Vec<_>>();
    let parameter_signals = schema
        .methods()
        .iter()
        .filter(|method| {
            method.documentation().tags().iter().any(|tag| {
                tag.name() != "description"
                    && field_type(method, tag.name()).is_some()
                    && has_runtime_gate_signal(&normalized_text(tag.value()))
            })
        })
        .map(|method| method.name())
        .collect::<Vec<_>>();
    assert_eq!(
        hash_method_set(description_signals),
        (
            162,
            "f0ce76fbe2c80365483306b65f1334fdc20a6ad93d956d32aec45c0c1d3b99fa".to_owned()
        ),
        "description runtime-signal set drift"
    );
    assert_eq!(
        hash_method_set(parameter_signals),
        (
            42,
            "aff7c31486573fe7c5d3c5b3fb586e1499d0816f6c9b857c1d48700c415deb9b".to_owned()
        ),
        "parameter runtime-signal set drift"
    );

    let mut supported = Vec::new();
    let mut unsupported = signaled
        .into_iter()
        .filter_map(|method| match documented_runtime_requirements(method) {
            Ok(_) => {
                supported.push(method.name());
                None
            }
            Err(error) => {
                assert_eq!(
                    error.kind(),
                    CapabilityGenerationErrorKind::SchemaDrift,
                    "unexpected disposition failure for {}: {error}",
                    method.name()
                );
                assert!(
                    error
                        .to_string()
                        .contains("unsupported runtime documentation"),
                    "unexpected disposition failure for {}: {error}",
                    method.name()
                );
                Some(method.name())
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(
        hash_method_set(supported),
        (
            5,
            "fe9034c9b7022707b3b29090ea6891209130cfb1b3acf69642bfbab652ee286d".to_owned()
        ),
        "reviewed real runtime-contract set drift"
    );
    unsupported.sort_unstable();
    let mut unsupported_oracle = unsupported.join("\n");
    unsupported_oracle.push('\n');
    assert_eq!(
        unsupported.len(),
        188,
        "reviewed runtime-disposition boundary drift"
    );
    assert_eq!(
        sha256_hex(unsupported_oracle.as_bytes()),
        "c9e5131cd86d5ebe7eb697f409953d4090c58a4c21ba9e442075701c6d950a34",
        "reviewed runtime-disposition boundary hash drift"
    );
}

#[test]
fn requires_an_exact_capability_partition() {
    let fixture = Fixture::new(SCHEMA);
    let baseline = fixture.capability_value();

    let mut missing = baseline.clone();
    missing["methods"].as_array_mut().unwrap().pop();
    assert_policy_error(&fixture, missing, CapabilityGenerationErrorKind::Coverage);

    let mut duplicate = baseline.clone();
    let row = duplicate["methods"][0].clone();
    duplicate["methods"].as_array_mut().unwrap().push(row);
    assert_policy_error(
        &fixture,
        duplicate,
        CapabilityGenerationErrorKind::InvalidPolicy,
    );

    let mut unknown = baseline;
    unknown["methods"][0]["method"] = json!("unknownMethod");
    assert_policy_error(&fixture, unknown, CapabilityGenerationErrorKind::Coverage);
}

#[test]
fn enforces_account_entitlement_and_set_invariants() {
    let fixture = Fixture::new(SCHEMA);
    let cases: Vec<(&str, PolicyMutation)> = vec![
        (
            "Ready without account",
            Box::new(|policy| method_row_mut(policy, "getChat")["ready_accounts"] = json!([])),
        ),
        (
            "pre-auth with account",
            Box::new(|policy| {
                method_row_mut(policy, "setTdlibParameters")["ready_accounts"] = json!(["bot"])
            }),
        ),
        (
            "Premium for bot",
            Box::new(|policy| {
                let row = method_row_mut(policy, "usePremiumFeature");
                row["ready_accounts"] = json!(["bot"]);
            }),
        ),
        (
            "Premium method widened to Business",
            Box::new(|policy| {
                method_row_mut(policy, "usePremiumFeature")["current_account_entitlements"] =
                    json!(["premium", "business"])
            }),
        ),
        (
            "Business method widened to Premium",
            Box::new(|policy| {
                method_row_mut(policy, "useBusinessFeature")["current_account_entitlements"] =
                    json!(["business", "premium"])
            }),
        ),
        (
            "official method narrowed to mobile",
            Box::new(|policy| {
                method_row_mut(policy, "submitStorePayment")["application"] =
                    json!("official_mobile")
            }),
        ),
        (
            "ordinary method narrowed to regular user",
            Box::new(|policy| {
                method_row_mut(policy, "getChat")["ready_accounts"] = json!(["regular_user"])
            }),
        ),
        (
            "ordinary method narrowed to Premium",
            Box::new(|policy| {
                let row = method_row_mut(policy, "getChat");
                row["ready_accounts"] = json!(["regular_user"]);
                row["current_account_entitlements"] = json!(["premium"]);
            }),
        ),
        (
            "ordinary method narrowed to official mobile",
            Box::new(|policy| {
                method_row_mut(policy, "getChat")["application"] = json!("official_mobile")
            }),
        ),
        (
            "ordinary method narrowed to Test DC",
            Box::new(|policy| {
                method_row_mut(policy, "getChat")["dc_environments"] = json!(["test"])
            }),
        ),
        (
            "empty auth set",
            Box::new(|policy| {
                method_row_mut(policy, "getChat")["authorization_states"] = json!([])
            }),
        ),
        (
            "empty DC set",
            Box::new(|policy| method_row_mut(policy, "getChat")["dc_environments"] = json!([])),
        ),
        (
            "duplicate enum value",
            Box::new(|policy| {
                method_row_mut(policy, "getChat")["ready_accounts"] =
                    json!(["regular_user", "regular_user"])
            }),
        ),
    ];
    for (name, mutate) in cases {
        let mut policy = fixture.capability_value();
        mutate(&mut policy);
        let error = expect_generation_error(&fixture, &policy, name);
        assert_eq!(
            error.kind(),
            CapabilityGenerationErrorKind::InvalidPolicy,
            "{name}"
        );
    }
}

#[test]
fn validates_runtime_alternatives_rights_and_argument_types() {
    let fixture = Fixture::new(SCHEMA);
    let cases: Vec<(&str, Value)> = vec![
        (
            "empty alternatives",
            json!({"kind": "any_of", "clauses": []}),
        ),
        (
            "empty clause",
            json!({"kind": "any_of", "clauses": [{"all_of": []}]}),
        ),
        (
            "duplicate atom",
            json!({"kind": "any_of", "clauses": [{"all_of": [
                {"kind": "chat_owner", "target_argument": "chat_id"},
                {"kind": "chat_owner", "target_argument": "chat_id"}
            ]}]}),
        ),
        (
            "duplicate clause",
            json!({"kind": "any_of", "clauses": [
                {"all_of": [{"kind": "chat_owner", "target_argument": "chat_id"}]},
                {"all_of": [{"kind": "chat_owner", "target_argument": "chat_id"}]}
            ]}),
        ),
        (
            "unknown right",
            json!({"kind": "any_of", "clauses": [{"all_of": [{
                "kind": "chat_member_right", "target_argument": "chat_id", "right": "can_fly"
            }]}]}),
        ),
        (
            "missing argument",
            json!({"kind": "any_of", "clauses": [{"all_of": [{
                "kind": "chat_owner", "target_argument": "missing_id"
            }]}]}),
        ),
        (
            "wrong argument type",
            json!({"kind": "any_of", "clauses": [{"all_of": [{
                "kind": "chat_owner", "target_argument": "reason"
            }]}]}),
        ),
        (
            "wrong same-type semantic argument",
            json!({"kind": "any_of", "clauses": [{"all_of": [{
                "kind": "chat_owner", "target_argument": "message_id"
            }]}]}),
        ),
    ];
    for (name, requirements) in cases {
        let mut policy = fixture.capability_value();
        method_row_mut(&mut policy, "unpinChatMessage")["runtime_requirements"] = requirements;
        let error = expect_generation_error(&fixture, &policy, name);
        assert_eq!(
            error.kind(),
            CapabilityGenerationErrorKind::InvalidPolicy,
            "{name}"
        );
    }

    let mut wrong_business_role = fixture.capability_value();
    method_row_mut(&mut wrong_business_role, "sendBusinessMessage")["runtime_requirements"] = json!({"kind": "any_of", "clauses": [{"all_of": [{
        "kind": "business_connection_enabled", "connection_argument": "reason"
    }]}]});
    assert_policy_error(
        &fixture,
        wrong_business_role,
        CapabilityGenerationErrorKind::InvalidPolicy,
    );

    let mut wrong_topic_role = fixture.capability_value();
    method_row_mut(&mut wrong_topic_role, "toggleForumTopicIsClosed")["runtime_requirements"] = json!({"kind": "any_of", "clauses": [
        {"all_of": [{
            "kind": "chat_administrator_right",
            "target_argument": "chat_id",
            "right": "can_manage_topics"
        }]},
        {"all_of": [{
            "kind": "topic_creator",
            "target_argument": "chat_id",
            "topic_argument": "other_topic_id"
        }]}
    ]});
    assert_policy_error(
        &fixture,
        wrong_topic_role,
        CapabilityGenerationErrorKind::InvalidPolicy,
    );

    let mut wrong_supergroup_role = fixture.capability_value();
    method_row_mut(&mut wrong_supergroup_role, "setSupergroupStickerSet")["runtime_requirements"] = json!({"kind": "any_of", "clauses": [{"all_of": [{
        "kind": "chat_administrator_right",
        "target_argument": "other_supergroup_id",
        "right": "can_change_info"
    }]}]});
    assert_policy_error(
        &fixture,
        wrong_supergroup_role,
        CapabilityGenerationErrorKind::InvalidPolicy,
    );
}

#[test]
fn rejects_omitted_documented_runtime_requirements() {
    let fixture = Fixture::new(SCHEMA);
    for method in [
        "unpinChatMessage",
        "toggleForumTopicIsClosed",
        "setSupergroupStickerSet",
        "requireSyntheticSupergroupAdministrator",
        "upgradeBasicGroupChatToSupergroupChat",
    ] {
        let mut policy = fixture.capability_value();
        method_row_mut(&mut policy, method)["runtime_requirements"] = json!({"kind": "always"});
        let error = fixture.generate_value(&policy).expect_err(method);
        assert_eq!(error.kind(), CapabilityGenerationErrorKind::InvalidPolicy);
        assert!(
            error.to_string().contains("runtime requirements"),
            "{method}: {error}"
        );
    }

    let mut missing_topic_branch = fixture.capability_value();
    method_row_mut(&mut missing_topic_branch, "toggleForumTopicIsClosed")["runtime_requirements"]
        ["clauses"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert_policy_error(
        &fixture,
        missing_topic_branch,
        CapabilityGenerationErrorKind::InvalidPolicy,
    );

    let mut undocumented_extra = fixture.capability_value();
    method_row_mut(&mut undocumented_extra, "getChat")["runtime_requirements"] = json!({
        "kind": "any_of",
        "clauses": [{"all_of": [{
            "kind": "chat_owner",
            "target_argument": "chat_id"
        }]}]
    });
    assert_policy_error(
        &fixture,
        undocumented_extra,
        CapabilityGenerationErrorKind::InvalidPolicy,
    );
}

#[test]
fn validates_parameter_notices_and_all_orthogonal_gate_axes() {
    let fixture = Fixture::new(SCHEMA);
    let mut missing = fixture.capability_value();
    method_row_mut(&mut missing, "configureGatedValues")["parameter_notices"]
        .as_array_mut()
        .unwrap()
        .retain(|notice| notice["parameter"] != "official_mobile_value");
    assert_policy_error(
        &fixture,
        missing,
        CapabilityGenerationErrorKind::InvalidPolicy,
    );

    let mut unknown_argument = fixture.capability_value();
    method_row_mut(&mut unknown_argument, "configureGatedValues")["parameter_notices"][0]["parameter"] =
        json!("missing_value");
    assert_policy_error(
        &fixture,
        unknown_argument,
        CapabilityGenerationErrorKind::InvalidPolicy,
    );

    let mut duplicate = fixture.capability_value();
    let notice =
        method_row_mut(&mut duplicate, "configureGatedValues")["parameter_notices"][0].clone();
    method_row_mut(&mut duplicate, "configureGatedValues")["parameter_notices"]
        .as_array_mut()
        .unwrap()
        .push(notice);
    assert_policy_error(
        &fixture,
        duplicate,
        CapabilityGenerationErrorKind::InvalidPolicy,
    );

    let mut unknown_enum = fixture.capability_value();
    method_row_mut(&mut unknown_enum, "configureGatedValues")["parameter_notices"][0]["gate"]["value"] =
        json!("service_account");
    assert_policy_error(
        &fixture,
        unknown_enum,
        CapabilityGenerationErrorKind::InvalidPolicy,
    );

    let mut undocumented = fixture.capability_value();
    method_row_mut(&mut undocumented, "unpinChatMessage")["parameter_notices"] = json!([{
        "parameter": "reason",
        "gate": {"kind": "account", "value": "bot"}
    }]);
    assert_policy_error(
        &fixture,
        undocumented,
        CapabilityGenerationErrorKind::InvalidPolicy,
    );

    let mut extra_gate = fixture.capability_value();
    method_row_mut(&mut extra_gate, "configureGatedValues")["parameter_notices"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "parameter": "bot_value",
            "gate": {"kind": "current_account_entitlement", "value": "premium"}
        }));
    assert_policy_error(
        &fixture,
        extra_gate,
        CapabilityGenerationErrorKind::InvalidPolicy,
    );

    let mut wrong_gate = fixture.capability_value();
    method_row_mut(&mut wrong_gate, "configureGatedValues")["parameter_notices"][0]["gate"] =
        json!({"kind": "account", "value": "regular_user"});
    assert_policy_error(
        &fixture,
        wrong_gate,
        CapabilityGenerationErrorKind::InvalidPolicy,
    );
}

#[test]
fn method_level_official_mobile_implies_parameter_level_official() {
    let schema = Schema::parse(
        r#"string ? = String;
ok = Ok;

---functions---

//@description Uses an official-only value; for official mobile applications only @value Value available to official applications only
useOfficialMobileValue value:string = Ok;
"#,
    )
    .expect("synthetic schema");
    let descriptor = CapabilityDescriptor::try_new(
        SynchronousCapability::Never,
        vec![AccountKind::RegularUser, AccountKind::Bot],
        vec![AuthorizationState::Ready],
        Vec::new(),
        ApplicationRequirement::OfficialMobile,
        vec![DcEnvironment::Production, DcEnvironment::Test],
        RequirementAlternatives::always(),
        Vec::new(),
    )
    .expect("official-mobile method descriptor");

    validate_documented_parameter_notices(
        find_method(&schema, "useOfficialMobileValue"),
        &descriptor,
    )
    .expect("method-level official-mobile access already implies official parameter access");
}

#[test]
fn synchronous_capability_is_additive_and_value_conditioned() {
    let fixture = Fixture::new(SCHEMA);

    let mut lost_sync = fixture.capability_value();
    method_row_mut(&mut lost_sync, "parseText")["synchronous"] = json!({"kind": "never"});
    assert_policy_error(
        &fixture,
        lost_sync,
        CapabilityGenerationErrorKind::InvalidPolicy,
    );

    let mut widened = fixture.capability_value();
    method_row_mut(&mut widened, "getOption")["synchronous"] = json!({"kind": "always"});
    assert_policy_error(
        &fixture,
        widened,
        CapabilityGenerationErrorKind::InvalidPolicy,
    );

    let mut wrong_values = fixture.capability_value();
    method_row_mut(&mut wrong_values, "getOption")["synchronous"]["values"] =
        json!(["version", "other"]);
    assert_policy_error(
        &fixture,
        wrong_values,
        CapabilityGenerationErrorKind::InvalidPolicy,
    );
}

#[test]
fn binds_every_row_to_documentation_signature_and_owner_evidence() {
    let fixture = Fixture::new(SCHEMA);

    for (field, kind) in [
        (
            "documentation_sha256",
            CapabilityGenerationErrorKind::SchemaDrift,
        ),
        (
            "signature_sha256",
            CapabilityGenerationErrorKind::SchemaDrift,
        ),
        ("feature_id", CapabilityGenerationErrorKind::OwnerDrift),
    ] {
        let mut policy = fixture.capability_value();
        method_row_mut(&mut policy, "getChat")[field] = if field == "feature_id" {
            json!("F017")
        } else {
            json!("0".repeat(64))
        };
        assert_policy_error(&fixture, policy, kind);
    }

    let mut root_owner = fixture.capability_value();
    root_owner["owner_mapping_sha256"] = json!("0".repeat(64));
    assert_policy_error(
        &fixture,
        root_owner,
        CapabilityGenerationErrorKind::OwnerDrift,
    );
}

#[test]
fn rejects_unknown_fields_and_oversized_inputs_before_work() {
    let fixture = Fixture::new(SCHEMA);
    for mutate in [
        |policy: &mut Value| policy["priority"] = json!(1),
        |policy: &mut Value| method_row_mut(policy, "getChat")["priority"] = json!(1),
        |policy: &mut Value| {
            method_row_mut(policy, "unpinChatMessage")["runtime_requirements"]["clauses"][0]["all_of"]
                [0]["probe"] = json!(true)
        },
        |policy: &mut Value| {
            method_row_mut(policy, "configureGatedValues")["parameter_notices"][0]["gate"]["unexpected"] =
                json!(true)
        },
    ] {
        let mut unknown = fixture.capability_value();
        mutate(&mut unknown);
        assert_policy_error(
            &fixture,
            unknown,
            CapabilityGenerationErrorKind::InvalidPolicy,
        );
    }

    let mut enum_cap = fixture.capability_value();
    method_row_mut(&mut enum_cap, "getChat")["ready_accounts"] =
        json!(["regular_user", "bot", "bot"]);
    assert_policy_error(
        &fixture,
        enum_cap,
        CapabilityGenerationErrorKind::ResourceLimit,
    );

    let mut synchronous_cap = fixture.capability_value();
    method_row_mut(&mut synchronous_cap, "getOption")["synchronous"]["values"] =
        json!(vec!["version"; 17]);
    assert_policy_error(
        &fixture,
        synchronous_cap,
        CapabilityGenerationErrorKind::ResourceLimit,
    );

    let atom = json!({"kind": "chat_owner", "target_argument": "chat_id"});
    let mut atom_cap = fixture.capability_value();
    method_row_mut(&mut atom_cap, "unpinChatMessage")["runtime_requirements"] = json!({
        "kind": "any_of", "clauses": [{"all_of": vec![atom.clone(); 33]}]
    });
    assert_policy_error(
        &fixture,
        atom_cap,
        CapabilityGenerationErrorKind::ResourceLimit,
    );

    let mut clause_cap = fixture.capability_value();
    method_row_mut(&mut clause_cap, "unpinChatMessage")["runtime_requirements"] = json!({
        "kind": "any_of", "clauses": vec![json!({"all_of": [atom]}); 17]
    });
    assert_policy_error(
        &fixture,
        clause_cap,
        CapabilityGenerationErrorKind::ResourceLimit,
    );

    let mut notice_cap = fixture.capability_value();
    let notice = notice("bot_value", "account", "bot");
    method_row_mut(&mut notice_cap, "configureGatedValues")["parameter_notices"] =
        json!(vec![notice; 33]);
    assert_policy_error(
        &fixture,
        notice_cap,
        CapabilityGenerationErrorKind::ResourceLimit,
    );

    for (manifest, schema, owner, capability) in [
        (
            vec![b' '; MAX_MANIFEST_BYTES + 1],
            fixture.schema.clone(),
            fixture.owner_policy.clone(),
            fixture.capability_policy.clone(),
        ),
        (
            fixture.manifest.clone(),
            vec![b' '; MAX_SCHEMA_BYTES + 1],
            fixture.owner_policy.clone(),
            fixture.capability_policy.clone(),
        ),
        (
            fixture.manifest.clone(),
            fixture.schema.clone(),
            vec![b' '; MAX_OWNER_POLICY_BYTES + 1],
            fixture.capability_policy.clone(),
        ),
        (
            fixture.manifest.clone(),
            fixture.schema.clone(),
            fixture.owner_policy.clone(),
            vec![b' '; MAX_CAPABILITY_POLICY_BYTES + 1],
        ),
    ] {
        assert_eq!(
            generate(&manifest, &schema, &owner, &capability)
                .expect_err("input cap")
                .kind(),
            CapabilityGenerationErrorKind::ResourceLimit
        );
    }

    assert_eq!(
        serialize_pretty_with_limit(&json!({"bounded": "output"}), 1)
            .expect_err("output cap")
            .kind(),
        CapabilityGenerationErrorKind::ResourceLimit
    );
}

#[test]
fn generated_artifact_contains_requirements_but_no_runtime_claims() {
    let artifact: Value =
        serde_json::from_slice(&Fixture::new(SCHEMA).generate().unwrap()).unwrap();
    let encoded = serde_json::to_string(&artifact).unwrap();
    for forbidden in [
        "available",
        "satisfied",
        "has_right",
        "premium_active",
        "current_account_id",
    ] {
        assert!(!encoded.contains(&format!("\"{forbidden}\"")));
    }
    assert_eq!(artifact["generated_by"], "tdlib-registry-gen/capability");
}

#[test]
fn recognizers_reject_unclassified_constraints_from_the_real_pinned_wording() {
    let schema =
        Schema::parse(include_str!("../../../../vendor/tdlib/td_api.tl")).expect("pinned schema");
    let unrestricted = ready_descriptor(ApplicationRequirement::Any);
    for method in [
        "searchSavedMessages",
        "sendQuickReplyShortcutMessages",
        "checkAuthenticationWebToken",
        "reportAuthenticationCodeMissing",
    ] {
        let error =
            validate_documented_method_constraints(find_method(&schema, method), &unrestricted)
                .expect_err(method);
        assert_eq!(error.kind(), CapabilityGenerationErrorKind::InvalidPolicy);
    }

    for method in [
        "sendMessage",
        "createForumTopic",
        "createChatSubscriptionInviteLink",
        "sendGiftPurchaseOffer",
        "postStory",
        "reorderChatFolders",
    ] {
        let error =
            validate_documented_parameter_notices(find_method(&schema, method), &unrestricted)
                .expect_err(method);
        assert_eq!(error.kind(), CapabilityGenerationErrorKind::InvalidPolicy);
    }

    let bot_only = CapabilityDescriptor::try_new(
        SynchronousCapability::Never,
        vec![AccountKind::Bot],
        vec![AuthorizationState::Ready],
        Vec::new(),
        ApplicationRequirement::Any,
        vec![DcEnvironment::Production, DcEnvironment::Test],
        RequirementAlternatives::always(),
        Vec::new(),
    )
    .expect("bot-only descriptor");
    validate_documented_parameter_notices(find_method(&schema, "sendMessage"), &bot_only)
        .expect_err("a parameter-only bot gate can't be lifted to the whole method");

    let premium_only = CapabilityDescriptor::try_new(
        SynchronousCapability::Never,
        vec![AccountKind::RegularUser],
        vec![AuthorizationState::Ready],
        vec![telegram_core::method_capability::CurrentAccountEntitlement::Premium],
        ApplicationRequirement::Any,
        vec![DcEnvironment::Production, DcEnvironment::Test],
        RequirementAlternatives::always(),
        Vec::new(),
    )
    .expect("Premium-only descriptor");
    validate_documented_parameter_notices(find_method(&schema, "createForumTopic"), &premium_only)
        .expect_err("a parameter-only Premium gate can't be lifted to the whole method");

    for method in [
        "setTdlibParameters",
        "getOption",
        "getAuthorizationState",
        "destroy",
    ] {
        let error =
            validate_documented_authorization_states(find_method(&schema, method), &unrestricted)
                .expect_err(method);
        assert_eq!(error.kind(), CapabilityGenerationErrorKind::InvalidPolicy);
    }

    for method in [
        "deleteChatMessagesBySender",
        "addChatMember",
        "toggleForumTopicIsClosed",
        "upgradeBasicGroupChatToSupergroupChat",
        "setSupergroupStickerSet",
    ] {
        let error =
            validate_documented_runtime_requirements(find_method(&schema, method), &unrestricted)
                .expect_err(method);
        assert_eq!(error.kind(), CapabilityGenerationErrorKind::InvalidPolicy);
    }

    for method in [
        "unpinChatMessage",
        "createForumTopic",
        "editForumTopic",
        "deleteForumTopic",
        "setChatMemberStatus",
        "reportSupergroupSpam",
        "getSupergroupMembers",
        "banGroupCallParticipants",
        "canPostStory",
        "postStory",
        "startLiveStory",
        "setChatBackground",
        "toggleChatHasProtectedContent",
        "setChatMemberTag",
        "setChatEmojiStatus",
        "setSupergroupCustomEmojiStickerSet",
        "toggleSupergroupCanHaveSponsoredMessages",
        "toggleSupergroupHasAutomaticTranslation",
        "toggleSupergroupJoinByRequest",
        "reportSupergroupAntiSpamFalsePositive",
        "setNewChatPrivacySettings",
    ] {
        let error =
            validate_documented_runtime_requirements(find_method(&schema, method), &unrestricted)
                .expect_err(method);
        assert_eq!(error.kind(), CapabilityGenerationErrorKind::SchemaDrift);
        assert!(
            error
                .to_string()
                .contains("unsupported runtime documentation")
        );
    }

    for method in [
        "setDefaultGroupAdministratorRights",
        "setDefaultChannelAdministratorRights",
    ] {
        validate_documented_runtime_requirements(find_method(&schema, method), &unrestricted)
            .unwrap_or_else(|error| panic!("{method} has no current-chat runtime gate: {error}"));
    }

    validate_documented_parameter_notices(
        find_method(&schema, "setTdlibParameters"),
        &unrestricted,
    )
    .unwrap_or_else(|error| {
        panic!("use_test_dc selects an environment; it isn't a gated value: {error}")
    });

    let guard_parameter_only_source = include_str!("../../../../vendor/tdlib/td_api.tl").replace(
        "Toggles whether all users directly joining the supergroup need to be approved by supergroup administrators; requires can_restrict_members administrator right",
        "Toggles whether all users directly joining the supergroup need to be approved",
    );
    let guard_parameter_only = Schema::parse(&guard_parameter_only_source).expect("mutated schema");
    let error = validate_documented_runtime_requirements(
        find_method(&guard_parameter_only, "toggleSupergroupJoinByRequest"),
        &unrestricted,
    )
    .expect_err("guard-bot parameter requirements can't be silently ignored");
    assert_eq!(error.kind(), CapabilityGenerationErrorKind::SchemaDrift);

    let option_parameter_only_source = include_str!("../../../../vendor/tdlib/td_api.tl").replace(
        "Posts a new story on behalf of a chat; requires can_post_stories administrator right for supergroup and channel chats. Returns a temporary story",
        "Posts a new story",
    );
    let option_parameter_only =
        Schema::parse(&option_parameter_only_source).expect("mutated schema");
    let error = validate_documented_runtime_requirements(
        find_method(&option_parameter_only, "postStory"),
        &unrestricted,
    )
    .expect_err("getOption-gated parameter values can't be silently ignored");
    assert_eq!(error.kind(), CapabilityGenerationErrorKind::SchemaDrift);

    let passkey_states = CapabilityDescriptor::try_new(
        SynchronousCapability::Never,
        Vec::new(),
        vec![
            AuthorizationState::WaitPhoneNumber,
            AuthorizationState::WaitOtherDeviceConfirmation,
            AuthorizationState::WaitPremiumPurchase,
            AuthorizationState::WaitEmailAddress,
            AuthorizationState::WaitEmailCode,
            AuthorizationState::WaitCode,
            AuthorizationState::WaitRegistration,
            AuthorizationState::WaitPassword,
        ],
        Vec::new(),
        ApplicationRequirement::Any,
        vec![DcEnvironment::Production, DcEnvironment::Test],
        RequirementAlternatives::always(),
        Vec::new(),
    )
    .expect("reviewed passkey cross-reference has an exact pre-auth state set");
    validate_documented_authorization_states(
        find_method(&schema, "getAuthenticationPasskeyParameters"),
        &passkey_states,
    )
    .expect("reviewed passkey exception is exact and schema-bound");

    let destroy_states = CapabilityDescriptor::try_new(
        SynchronousCapability::Never,
        vec![AccountKind::RegularUser, AccountKind::Bot],
        post_initialization_authorization_states()
            .into_iter()
            .map(|state| AuthorizationState::try_from(state).unwrap())
            .collect(),
        Vec::new(),
        ApplicationRequirement::Any,
        vec![DcEnvironment::Production, DcEnvironment::Test],
        RequirementAlternatives::always(),
        Vec::new(),
    )
    .expect("destroy is available before authorization, but only after initialization");
    validate_documented_authorization_states(find_method(&schema, "destroy"), &destroy_states)
        .expect("reviewed destroy exception excludes WaitTdlibParameters");
}

fn ready_descriptor(application: ApplicationRequirement) -> CapabilityDescriptor {
    CapabilityDescriptor::try_new(
        SynchronousCapability::Never,
        vec![AccountKind::RegularUser, AccountKind::Bot],
        vec![AuthorizationState::Ready],
        Vec::new(),
        application,
        vec![DcEnvironment::Production, DcEnvironment::Test],
        RequirementAlternatives::always(),
        Vec::new(),
    )
    .expect("unrestricted Ready descriptor")
}

fn find_method<'a>(schema: &'a Schema, name: &str) -> &'a telegram_core::schema::Definition {
    schema
        .methods()
        .iter()
        .find(|method| method.name() == name)
        .unwrap_or_else(|| panic!("missing pinned method {name}"))
}

struct Fixture {
    manifest: Vec<u8>,
    schema: Vec<u8>,
    owner_policy: Vec<u8>,
    capability_policy: Vec<u8>,
}

impl Fixture {
    fn new(schema_source: &str) -> Self {
        let schema = Schema::parse(schema_source).expect("fixture schema");
        let schema_bytes = schema_source.as_bytes().to_vec();
        let inventory = schema.inventory();
        let manifest = serde_json::to_vec(&json!({
            "format_version": 1,
            "upstream": {
                "repository": "https://example.invalid/tdlib",
                "commit": "0123456789abcdef0123456789abcdef01234567",
                "version": "test"
            },
            "schema": {
                "sha256": sha256_hex(&schema_bytes),
                "definitions": schema.definitions().len(),
                "functions": schema.methods().len(),
                "updates": inventory.update_names().len(),
                "authorization_states": inventory.authorization_state_names().len()
            }
        }))
        .unwrap();
        let owner_policy = owner_policy(&schema, &schema_bytes);
        let owner_output = super::engine::generate(&manifest, &schema_bytes, &owner_policy)
            .expect("fixture owner output");
        let owner: Value = serde_json::from_slice(&owner_output).unwrap();
        let owner_by_method = owner["methods"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                (
                    row["method"].as_str().unwrap(),
                    row["feature_id"].as_str().unwrap(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let methods = schema
            .methods()
            .iter()
            .map(|method| capability_row(method, owner_by_method[method.name()]))
            .collect::<Vec<_>>();
        let capability_policy = serde_json::to_vec(&json!({
            "format_version": 1,
            "schema_sha256": sha256_hex(&schema_bytes),
            "owner_mapping_sha256": owner["mapping_sha256"],
            "methods": methods
        }))
        .unwrap();
        Self {
            manifest,
            schema: schema_bytes,
            owner_policy,
            capability_policy,
        }
    }

    fn capability_value(&self) -> Value {
        serde_json::from_slice(&self.capability_policy).unwrap()
    }

    fn generate(&self) -> Result<Vec<u8>, super::CapabilityGenerationError> {
        generate(
            &self.manifest,
            &self.schema,
            &self.owner_policy,
            &self.capability_policy,
        )
    }

    fn generate_value(&self, value: &Value) -> Result<Vec<u8>, super::CapabilityGenerationError> {
        generate(
            &self.manifest,
            &self.schema,
            &self.owner_policy,
            &serde_json::to_vec(value).unwrap(),
        )
    }
}

fn capability_row(method: &telegram_core::schema::Definition, feature_id: &str) -> Value {
    let common = || {
        json!({
            "method": method.name(),
            "signature_sha256": sha256_hex(method.canonical_signature().as_bytes()),
            "documentation_sha256": documentation_sha256(method),
            "feature_id": feature_id,
            "synchronous": {"kind": "never"},
            "ready_accounts": ["regular_user", "bot"],
            "authorization_states": ["authorizationStateReady"],
            "current_account_entitlements": [],
            "application": "any",
            "dc_environments": ["production", "test"],
            "runtime_requirements": {"kind": "always"},
            "parameter_notices": [],
            "rationale": "Reviewed synthetic capability evidence."
        })
    };
    let mut row = common();
    match method.name() {
        "getAuthorizationState" => {
            row["authorization_states"] = json!(requestable_authorization_states());
        }
        "setTdlibParameters" => {
            row["ready_accounts"] = json!([]);
            row["authorization_states"] = json!(["authorizationStateWaitTdlibParameters"]);
        }
        "usePremiumFeature" => {
            row["ready_accounts"] = json!(["regular_user"]);
            row["current_account_entitlements"] = json!(["premium"]);
        }
        "useBusinessFeature" => {
            row["ready_accounts"] = json!(["regular_user"]);
            row["current_account_entitlements"] = json!(["business"]);
        }
        "submitStorePayment" => {
            row["ready_accounts"] = json!(["regular_user"]);
            row["application"] = json!("official");
        }
        "testNetworkRequest" => row["dc_environments"] = json!(["test"]),
        "unpinChatMessage" => {
            row["runtime_requirements"] = json!({
                "kind": "any_of",
                "clauses": [
                    {"all_of": [{
                        "kind": "chat_member_right",
                        "target_argument": "chat_id",
                        "right": "can_pin_messages"
                    }]},
                    {"all_of": [{
                        "kind": "chat_administrator_right",
                        "target_argument": "chat_id",
                        "right": "can_edit_messages"
                    }]}
                ]
            });
        }
        "toggleForumTopicIsClosed" => {
            row["runtime_requirements"] = json!({
                "kind": "any_of",
                "clauses": [
                    {"all_of": [{
                        "kind": "chat_administrator_right",
                        "target_argument": "chat_id",
                        "right": "can_manage_topics"
                    }]},
                    {"all_of": [{
                        "kind": "topic_creator",
                        "target_argument": "chat_id",
                        "topic_argument": "forum_topic_id"
                    }]}
                ]
            });
        }
        "setSupergroupStickerSet" => {
            row["runtime_requirements"] = json!({
                "kind": "any_of",
                "clauses": [{"all_of": [{
                    "kind": "chat_administrator_right",
                    "target_argument": "supergroup_id",
                    "right": "can_change_info"
                }]}]
            });
        }
        "requireSyntheticSupergroupAdministrator" => {
            row["runtime_requirements"] = json!({
                "kind": "any_of",
                "clauses": [{"all_of": [{
                    "kind": "chat_administrator",
                    "target_argument": "supergroup_id"
                }]}]
            });
        }
        "upgradeBasicGroupChatToSupergroupChat" => {
            row["ready_accounts"] = json!(["regular_user"]);
            row["runtime_requirements"] = json!({
                "kind": "any_of",
                "clauses": [{"all_of": [{
                    "kind": "chat_owner",
                    "target_argument": "chat_id"
                }]}]
            });
        }
        "sendBusinessMessage" => {
            row["ready_accounts"] = json!(["bot"]);
            row["runtime_requirements"] = json!({
                "kind": "any_of",
                "clauses": [{"all_of": [
                    {
                        "kind": "business_connection_enabled",
                        "connection_argument": "business_connection_id"
                    },
                    {
                        "kind": "business_connection_right",
                        "connection_argument": "business_connection_id",
                        "right": "can_reply"
                    }
                ]}]
            });
        }
        "configureGatedValues" => {
            row["parameter_notices"] = json!([
                notice("bot_value", "account", "bot"),
                notice("premium_value", "current_account_entitlement", "premium"),
                notice("business_value", "current_account_entitlement", "business"),
                notice("official_value", "application", "official"),
                notice("official_mobile_value", "application", "official_mobile"),
                notice("production_value", "dc_environment", "production"),
                notice("test_value", "dc_environment", "test")
            ]);
        }
        "parseText" => row["synchronous"] = json!({"kind": "always"}),
        "getOption" => {
            row["authorization_states"] = json!(requestable_authorization_states());
            row["synchronous"] = json!({
                "kind": "string_parameter_values",
                "parameter": "name",
                "values": ["version", "commit_hash"]
            });
        }
        _ => {}
    }
    row
}

fn requestable_authorization_states() -> Vec<&'static str> {
    vec![
        "authorizationStateWaitTdlibParameters",
        "authorizationStateWaitPhoneNumber",
        "authorizationStateWaitPremiumPurchase",
        "authorizationStateWaitEmailAddress",
        "authorizationStateWaitEmailCode",
        "authorizationStateWaitCode",
        "authorizationStateWaitOtherDeviceConfirmation",
        "authorizationStateWaitRegistration",
        "authorizationStateWaitPassword",
        "authorizationStateReady",
    ]
}

fn post_initialization_authorization_states() -> Vec<&'static str> {
    requestable_authorization_states()
        .into_iter()
        .filter(|state| *state != "authorizationStateWaitTdlibParameters")
        .collect()
}

fn notice(parameter: &str, kind: &str, value: &str) -> Value {
    json!({"parameter": parameter, "gate": {"kind": kind, "value": value}})
}

fn owner_policy(schema: &Schema, schema_bytes: &[u8]) -> Vec<u8> {
    let (business, platform): (Vec<_>, Vec<_>) = schema
        .methods()
        .iter()
        .partition(|method| method.name() == "sendBusinessMessage");
    let rule = |feature_id: &str,
                methods: Vec<&telegram_core::schema::Definition>,
                positive: &str,
                negative: &str| {
        json!({
            "feature_id": feature_id,
            "any": methods.iter().map(|method| json!({
                "kind": "exact",
                "value": method.name()
            })).collect::<Vec<_>>(),
            "expected": {
                "method_count": methods.len(),
                "method_set_sha256": method_set_sha256(methods)
            },
            "positive_examples": [positive],
            "negative_examples": [negative],
            "rationale": "Synthetic owner boundary with exact method evidence."
        })
    };
    serde_json::to_vec(&json!({
        "format_version": 1,
        "schema_sha256": sha256_hex(schema_bytes),
        "rules": [
            rule("F017", business, "sendBusinessMessage", "getChat"),
            rule("F020", platform, "getChat", "sendBusinessMessage")
        ],
        "overrides": []
    }))
    .unwrap()
}

fn method_set_sha256(mut methods: Vec<&telegram_core::schema::Definition>) -> String {
    methods.sort_unstable_by_key(|method| method.name());
    let mut hasher = Sha256::new();
    for method in methods {
        hasher.update(method.name().as_bytes());
        hasher.update([0]);
        hasher.update(sha256_hex(method.canonical_signature().as_bytes()).as_bytes());
        hasher.update(b"\n");
    }
    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").unwrap();
    }
    encoded
}

fn reorder_policy(policy: &mut Value) {
    let methods = policy["methods"].as_array_mut().unwrap();
    methods.reverse();
    for row in methods {
        for field in [
            "ready_accounts",
            "authorization_states",
            "current_account_entitlements",
            "dc_environments",
            "parameter_notices",
        ] {
            row[field].as_array_mut().unwrap().reverse();
        }
        if let Some(values) = row["synchronous"].get_mut("values") {
            values.as_array_mut().unwrap().reverse();
        }
        if let Some(clauses) = row["runtime_requirements"].get_mut("clauses") {
            clauses.as_array_mut().unwrap().reverse();
            for clause in clauses.as_array_mut().unwrap() {
                clause["all_of"].as_array_mut().unwrap().reverse();
            }
        }
    }
}

fn method_row<'a>(artifact: &'a Value, method: &str) -> &'a Value {
    artifact["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["method"] == method)
        .unwrap()
}

fn method_row_mut<'a>(policy: &'a mut Value, method: &str) -> &'a mut Value {
    policy["methods"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["method"] == method)
        .unwrap()
}

fn assert_policy_error(fixture: &Fixture, policy: Value, expected: CapabilityGenerationErrorKind) {
    let error = expect_generation_error(fixture, &policy, "policy");
    assert_eq!(error.kind(), expected, "{error}");
}

fn expect_generation_error(
    fixture: &Fixture,
    policy: &Value,
    context: &str,
) -> super::CapabilityGenerationError {
    match fixture.generate_value(policy) {
        Ok(_) => panic!("{context}: policy unexpectedly generated"),
        Err(error) => error,
    }
}
