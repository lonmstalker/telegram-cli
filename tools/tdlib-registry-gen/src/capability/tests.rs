use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use telegram_core::method_capability::{
    AccountKind, ApplicationRequirement, AuthorizationState, CapabilityDescriptor,
    ChatAdministratorRight, ChatKindCondition, ChatMemberRight, ChatTargetKind, ChatTargetRef,
    DcEnvironment, ForumTopicRef, GroupCallMessageCapability, GroupCallMessageSubjectRef,
    GroupCallProperty, MessageCapability, MessageSubjectRef, RequirementAlternatives,
    ResolvedChatKind, ResolvedGroupCallKind, RuntimeRequirement, SupergroupFullInfoProperty,
    SynchronousCapability,
};
use telegram_core::schema::Schema;

use super::{
    CapabilityGenerationErrorKind, DeferredSignalLane, MAX_CAPABILITY_POLICY_BYTES,
    MAX_MANIFEST_BYTES, MAX_OWNER_POLICY_BYTES, MAX_SCHEMA_BYTES, NonGateReason,
    RuntimeRequirementsDto, RuntimeSignalDisposition, RuntimeSignalFamily, RuntimeSignalKey,
    RuntimeSignalSource, documentation_sha256, documented_runtime_requirements,
    documented_runtime_signal_dispositions, field_type, generate, has_runtime_gate_signal,
    method_documentation_text, normalized_text, parse_runtime_requirements,
    runtime_signal_dispositions_with_consumed, runtime_signal_families,
    serialize_pretty_with_limit, sha256_hex, validate_documented_authorization_states,
    validate_documented_method_constraints, validate_documented_parameter_notices,
    validate_documented_runtime_requirements, validate_group_call_vocabulary,
    validate_message_properties_vocabulary, validate_supergroup_full_info_vocabulary,
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

chatTypePrivate user_id:int53 = ChatType;
chatTypeBasicGroup basic_group_id:int53 = ChatType;
chatTypeSupergroup supergroup_id:int53 is_channel:Bool = ChatType;
chatTypeSecret secret_chat_id:int32 user_id:int53 = ChatType;

chatAdministratorRights is_anonymous:Bool can_manage_chat:Bool can_change_info:Bool can_post_messages:Bool can_edit_messages:Bool can_delete_messages:Bool can_invite_users:Bool can_restrict_members:Bool can_pin_messages:Bool can_manage_topics:Bool can_promote_members:Bool can_manage_video_chats:Bool can_post_stories:Bool can_edit_stories:Bool can_delete_stories:Bool can_manage_direct_messages:Bool can_manage_tags:Bool = ChatAdministratorRights;
chatPermissions can_send_basic_messages:Bool can_send_audios:Bool can_send_documents:Bool can_send_photos:Bool can_send_videos:Bool can_send_video_notes:Bool can_send_voice_notes:Bool can_send_polls:Bool can_send_other_messages:Bool can_add_link_previews:Bool can_react_to_messages:Bool can_edit_tag:Bool can_change_info:Bool can_invite_users:Bool can_pin_messages:Bool can_create_topics:Bool = ChatPermissions;
businessBotRights can_reply:Bool can_read_messages:Bool can_delete_sent_messages:Bool can_delete_all_messages:Bool can_edit_name:Bool can_edit_bio:Bool can_edit_profile_photo:Bool can_edit_username:Bool can_view_gifts_and_stars:Bool can_sell_gifts:Bool can_change_gift_settings:Bool can_transfer_and_upgrade_gifts:Bool can_transfer_stars:Bool can_manage_stories:Bool = BusinessBotRights;
messageProperties can_add_offer:Bool can_add_tasks:Bool can_be_approved:Bool can_be_copied:Bool can_be_copied_to_secret_chat:Bool can_be_declined:Bool can_be_deleted_only_for_self:Bool can_be_deleted_for_all_users:Bool can_be_edited:Bool can_be_forwarded:Bool can_be_paid:Bool can_be_pinned:Bool can_be_replied:Bool can_be_replied_in_another_chat:Bool can_be_saved:Bool can_be_shared_in_story:Bool can_delete_reactions:Bool can_edit_media:Bool can_edit_scheduling_state:Bool can_edit_suggested_post_info:Bool can_get_author:Bool can_get_embedding_code:Bool can_get_link:Bool can_get_media_timestamp_links:Bool can_get_message_thread:Bool can_get_poll_vote_statistics:Bool can_get_read_date:Bool can_get_statistics:Bool can_get_video_advertisements:Bool can_get_viewers:Bool can_mark_tasks_as_done:Bool can_recognize_speech:Bool can_report_chat:Bool can_report_reactions:Bool can_report_supergroup_spam:Bool can_set_fact_check:Bool has_protected_content_by_current_user:Bool has_protected_content_by_other_user:Bool need_show_statistics:Bool = MessageProperties;
messageSenderTest = MessageSender;
formattedText text:string = FormattedText;
groupCallRecentSpeaker participant_id:MessageSender is_speaking:Bool = GroupCallRecentSpeaker;
groupCall id:int32 unique_id:int64 title:string invite_link:string paid_message_star_count:int53 scheduled_start_date:int32 enabled_start_notification:Bool is_active:Bool is_video_chat:Bool is_live_story:Bool is_rtmp_stream:Bool is_joined:Bool need_rejoin:Bool is_owned:Bool can_be_managed:Bool participant_count:int32 has_hidden_listeners:Bool loaded_all_participants:Bool message_sender_id:MessageSender recent_speakers:vector<groupCallRecentSpeaker> is_my_video_enabled:Bool is_my_video_paused:Bool can_enable_video:Bool mute_new_participants:Bool can_toggle_mute_new_participants:Bool can_send_messages:Bool are_messages_allowed:Bool can_toggle_are_messages_allowed:Bool can_delete_messages:Bool record_duration:int32 is_video_recorded:Bool duration:int32 = GroupCall;
groupCallMessage message_id:int32 sender_id:MessageSender date:int32 text:formattedText paid_message_star_count:int53 is_from_owner:Bool can_be_deleted:Bool = GroupCallMessage;
chatPhoto = ChatPhoto;
chatLocation = ChatLocation;
chatInviteLink = ChatInviteLink;
botCommands = BotCommands;
botVerification = BotVerification;
profileTabTest = ProfileTab;
chatStatisticsTest = ChatStatistics;
supergroupFullInfo photo:chatPhoto community_id:int53 description:string member_count:int32 administrator_count:int32 restricted_count:int32 banned_count:int32 linked_chat_id:int53 direct_messages_chat_id:int53 slow_mode_delay:int32 slow_mode_delay_expires_in:double can_enable_paid_messages:Bool can_enable_paid_reaction:Bool can_get_members:Bool has_hidden_members:Bool can_hide_members:Bool can_set_sticker_set:Bool can_set_location:Bool can_get_statistics:Bool can_get_revenue_statistics:Bool can_get_star_revenue_statistics:Bool can_send_gift:Bool can_toggle_aggressive_anti_spam:Bool is_all_history_available:Bool can_have_sponsored_messages:Bool has_aggressive_anti_spam_enabled:Bool has_paid_media_allowed:Bool has_pinned_stories:Bool gift_count:int32 my_boost_count:int32 unrestrict_boost_count:int32 outgoing_paid_message_star_count:int53 sticker_set_id:int64 custom_emoji_sticker_set_id:int64 location:chatLocation invite_link:chatInviteLink guard_bot_user_id:int53 bot_commands:vector<botCommands> bot_verification:botVerification main_profile_tab:ProfileTab upgraded_from_basic_group_id:int53 upgraded_from_max_message_id:int53 = SupergroupFullInfo;
updateGroupCall group_call:groupCall = Update;
updateNewGroupCallMessage group_call_id:int32 message:groupCallMessage = Update;
updateGroupCallMessagesDeleted group_call_id:int32 message_ids:vector<int32> = Update;
updateSupergroupFullInfo supergroup_id:int53 supergroup_full_info:supergroupFullInfo = Update;

---functions---

//@description Returns the current authorization state. Can be called before initialization
getAuthorizationState = Ok;

//@description Provides initialization parameters. Works only when the current authorization state is authorizationStateWaitTdlibParameters
setTdlibParameters = Ok;

//@description Returns a chat @chat_id Chat identifier
getChat chat_id:int53 = Ok;

//@description Returns information about a group call @group_call_id Group call identifier
getGroupCall group_call_id:int32 = GroupCall;

//@description Returns properties of a message. This is an offline method @chat_id Chat identifier @message_id Identifier of the message
getMessageProperties chat_id:int53 message_id:int53 = MessageProperties;

//@description Returns full information about a supergroup or channel by its identifier @supergroup_id Identifier of the supergroup or channel
getSupergroupFullInfo supergroup_id:int53 = SupergroupFullInfo;

//@description Uses a feature; for Telegram Premium users only
usePremiumFeature = Ok;

//@description Uses a feature. Requires Telegram Business subscription
useBusinessFeature = Ok;

//@description Submits an in-store payment; for regular users only; for official Telegram apps only
submitStorePayment = Ok;

//@description Sends a request in Test DC only
testNetworkRequest = Ok;

//@description Removes a pinned message from a chat; requires can_pin_messages member right if the chat is a basic group or supergroup, or can_edit_messages administrator right if the chat is a channel @chat_id Chat identifier @message_id Message identifier @reason Diagnostic fixture
unpinChatMessage chat_id:int53 message_id:int53 reason:string = Ok;

//@description Toggles whether a topic is closed in a forum supergroup chat; requires can_manage_topics administrator right in the supergroup unless the user is creator of the topic @chat_id Chat identifier @forum_topic_id Forum topic identifier @other_topic_id Unrelated same-type identifier @is_closed New closed state
toggleForumTopicIsClosed chat_id:int53 forum_topic_id:int32 other_topic_id:int32 is_closed:Bool = Ok;

//@description Changes the sticker set of a supergroup; requires can_change_info administrator right @supergroup_id Identifier of the supergroup @other_supergroup_id Unrelated same-type identifier @sticker_set_id Sticker set identifier
setSupergroupStickerSet supergroup_id:int53 other_supergroup_id:int53 sticker_set_id:int53 = Ok;

//@description Requires administrator evidence in a synthetic supergroup fixture @supergroup_id Identifier of the supergroup
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
    assert_eq!(artifact["format_version"], super::FORMAT_VERSION);
    assert_eq!(artifact["counts"]["schema_methods"], 19);
    assert_eq!(artifact["counts"]["capability_methods"], 19);
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
    let unpin_clauses = unpin["runtime_requirements"]["clauses"]
        .as_array()
        .expect("unpin clauses");
    assert_eq!(unpin_clauses.len(), 5);
    assert_eq!(
        unpin_clauses
            .iter()
            .map(|clause| clause["all_of"][0]["value"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["private", "basic_group", "supergroup", "channel", "secret"]
    );
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
fn public_generation_enforces_and_serializes_message_capability_contracts() {
    let marker = "//@description Uses a feature; for Telegram Premium users only";
    let schema = SCHEMA.replacen(
        marker,
        concat!(
            "//@description Edits the text of a message\n",
            "//@chat_id The chat the message belongs to\n",
            "//@message_id Identifier of the message. Use messageProperties.can_be_edited to check whether the message can be edited\n",
            "editMessageText chat_id:int53 message_id:int53 = Ok;\n\n",
            "//@description Uses a feature; for Telegram Premium users only"
        ),
        1,
    );
    assert_ne!(schema, SCHEMA, "message-gated fixture insertion");
    let fixture = Fixture::new(&schema);
    let error = fixture
        .generate()
        .expect_err("public generator must reject omitted message capability");
    assert_eq!(error.kind(), CapabilityGenerationErrorKind::InvalidPolicy);

    let mut policy = fixture.capability_value();
    method_row_mut(&mut policy, "editMessageText")["runtime_requirements"] = json!({
        "kind": "any_of",
        "clauses": [{"all_of": [{
            "kind": "message_capability",
            "subject": {
                "kind": "one",
                "chat_argument": "chat_id",
                "message_argument": "message_id"
            },
            "capability": "can_be_edited"
        }]}]
    });
    let artifact: Value = serde_json::from_slice(
        &fixture
            .generate_value(&policy)
            .expect("public message-capability generation"),
    )
    .expect("canonical public artifact");
    assert_eq!(
        method_row(&artifact, "editMessageText")["runtime_requirements"],
        json!({
            "kind": "any_of",
            "clauses": [{"all_of": [{
                "kind": "message_capability",
                "subject": {
                    "kind": "one",
                    "chat": {"kind": "chat_id", "argument": "chat_id"},
                    "message_argument": "message_id"
                },
                "capability": "can_be_edited"
            }]}]
        })
    );
}

#[test]
fn public_generation_enforces_and_serializes_group_call_contracts() {
    let marker = "//@description Uses a feature; for Telegram Premium users only";
    let schema = SCHEMA.replacen(
        marker,
        concat!(
            "//@description Sets title of a video chat; requires groupCall.can_be_managed right\n",
            "//@group_call_id Group call identifier\n",
            "//@title New group call title; 1-64 characters\n",
            "setVideoChatTitle group_call_id:int32 title:string = Ok;\n\n",
            "//@description Uses a feature; for Telegram Premium users only"
        ),
        1,
    );
    assert_ne!(schema, SCHEMA, "group-call-gated fixture insertion");
    let fixture = Fixture::new(&schema);
    let error = fixture
        .generate()
        .expect_err("public generator must reject omitted group-call contract");
    assert_eq!(error.kind(), CapabilityGenerationErrorKind::InvalidPolicy);

    let mut property_only = fixture.capability_value();
    let row = method_row_mut(&mut property_only, "setVideoChatTitle");
    row["ready_accounts"] = json!(["regular_user"]);
    row["runtime_requirements"] = json!({
        "kind": "any_of",
        "clauses": [{"all_of": [{
            "kind": "group_call_property",
            "group_call_argument": "group_call_id",
            "property": "can_be_managed"
        }]}]
    });
    assert_eq!(
        fixture
            .generate_value(&property_only)
            .expect_err("video-only method needs a typed kind guard")
            .kind(),
        CapabilityGenerationErrorKind::InvalidPolicy
    );

    let mut policy = fixture.capability_value();
    let row = method_row_mut(&mut policy, "setVideoChatTitle");
    row["ready_accounts"] = json!(["regular_user"]);
    row["runtime_requirements"] = json!({
        "kind": "any_of",
        "clauses": [{"all_of": [
            {
                "kind": "group_call_kind",
                "group_call_argument": "group_call_id",
                "value": "video_chat"
            },
            {
                "kind": "group_call_property",
                "group_call_argument": "group_call_id",
                "property": "can_be_managed"
            }
        ]}]
    });
    let artifact: Value = serde_json::from_slice(
        &fixture
            .generate_value(&policy)
            .expect("public group-call capability generation"),
    )
    .expect("canonical public artifact");
    let row = method_row(&artifact, "setVideoChatTitle");
    assert_eq!(row["ready_accounts"], json!(["regular_user"]));
    assert_eq!(
        row["runtime_requirements"],
        json!({
            "kind": "any_of",
            "clauses": [{"all_of": [
                {
                    "kind": "group_call_kind",
                    "group_call_argument": "group_call_id",
                    "value": "video_chat"
                },
                {
                    "kind": "group_call_property",
                    "group_call_argument": "group_call_id",
                    "property": "can_be_managed"
                }
            ]}]
        })
    );
}

#[test]
fn public_generation_enforces_and_serializes_supergroup_full_info_contracts() {
    let marker = "//@description Uses a feature; for Telegram Premium users only";
    let schema = SCHEMA.replacen(
        marker,
        concat!(
            "//@description Returns detailed statistics about a chat. Currently, this method can be used only for supergroups and channels. Can be used only if supergroupFullInfo.can_get_statistics == true\n",
            "//@chat_id Chat identifier\n",
            "//@is_dark Pass true if a dark theme is used by the application\n",
            "getChatStatistics chat_id:int53 is_dark:Bool = ChatStatistics;\n\n",
            "//@description Uses a feature; for Telegram Premium users only"
        ),
        1,
    );
    assert_ne!(schema, SCHEMA, "full-info-gated fixture insertion");
    let fixture = Fixture::new(&schema);
    let mut omitted = fixture.capability_value();
    method_row_mut(&mut omitted, "getChatStatistics")["ready_accounts"] = json!(["regular_user"]);
    assert_eq!(
        fixture
            .generate_value(&omitted)
            .expect_err("public generator must reject omitted full-info contract")
            .kind(),
        CapabilityGenerationErrorKind::InvalidPolicy
    );

    let mut policy = fixture.capability_value();
    let row = method_row_mut(&mut policy, "getChatStatistics");
    row["ready_accounts"] = json!(["regular_user"]);
    row["runtime_requirements"] = json!({
        "kind": "any_of",
        "clauses": [{"all_of": [{
            "kind": "supergroup_full_info_property",
            "target_argument": "chat_id",
            "property": "can_get_statistics"
        }]}]
    });
    let artifact: Value = serde_json::from_slice(
        &fixture
            .generate_value(&policy)
            .expect("public full-info capability generation"),
    )
    .expect("canonical public artifact");
    let row = method_row(&artifact, "getChatStatistics");
    assert_eq!(row["ready_accounts"], json!(["regular_user"]));
    assert_eq!(
        row["runtime_requirements"],
        json!({
            "kind": "any_of",
            "clauses": [{"all_of": [{
                "kind": "supergroup_full_info_property",
                "target": {"kind": "chat_id", "argument": "chat_id"},
                "property": "can_get_statistics"
            }]}]
        })
    );
}

#[test]
fn public_generation_keeps_group_call_message_universal_cardinality() {
    let marker = "//@description Uses a feature; for Telegram Premium users only";
    let schema = SCHEMA.replacen(
        marker,
        concat!(
            "//@description Deletes messages in a group call; for live story calls only. Requires groupCallMessage.can_be_deleted right\n",
            "//@group_call_id Group call identifier\n",
            "//@message_ids Identifiers of the messages to be deleted\n",
            "//@report_spam Pass true to report the messages as spam\n",
            "deleteGroupCallMessages group_call_id:int32 message_ids:vector<int32> report_spam:Bool = Ok;\n\n",
            "//@description Uses a feature; for Telegram Premium users only"
        ),
        1,
    );
    let fixture = Fixture::new(&schema);
    let mut policy = fixture.capability_value();
    let row = method_row_mut(&mut policy, "deleteGroupCallMessages");
    row["ready_accounts"] = json!(["regular_user"]);
    row["runtime_requirements"] = json!({
        "kind": "any_of",
        "clauses": [{"all_of": [
            {
                "kind": "group_call_kind",
                "group_call_argument": "group_call_id",
                "value": "live_story"
            },
            {
                "kind": "group_call_message_capability",
                "subject": {
                    "kind": "each",
                    "group_call_argument": "group_call_id",
                    "message_argument": "message_ids"
                },
                "capability": "can_be_deleted"
            }
        ]}]
    });
    let artifact: Value = serde_json::from_slice(
        &fixture
            .generate_value(&policy)
            .expect("public universal group-call-message generation"),
    )
    .expect("canonical public artifact");
    assert_eq!(
        method_row(&artifact, "deleteGroupCallMessages")["runtime_requirements"],
        json!({
            "kind": "any_of",
            "clauses": [{"all_of": [
                {
                    "kind": "group_call_kind",
                    "group_call_argument": "group_call_id",
                    "value": "live_story"
                },
                {
                    "kind": "group_call_message_capability",
                    "subject": {
                        "kind": "each",
                        "group_call_argument": "group_call_id",
                        "message_argument": "message_ids"
                    },
                    "capability": "can_be_deleted"
                }
            ]}]
        })
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
fn requires_the_exact_pinned_chat_type_vocabulary() {
    for schema in [
        SCHEMA.replace("is_channel:Bool", "is_channel:int32"),
        SCHEMA.replace("chatTypeSecret secret_chat_id:int32 user_id:int53 = ChatType;\n", ""),
        SCHEMA.replace(
            "chatTypeSecret secret_chat_id:int32 user_id:int53 = ChatType;",
            "chatTypeSecret secret_chat_id:int32 user_id:int53 = ChatType;\nchatTypeUnknown = ChatType;",
        ),
        SCHEMA.replace(
            "chatTypeSecret secret_chat_id:int32 user_id:int53 = ChatType;",
            "chatTypeSecret user_id:int53 secret_chat_id:int32 = ChatType;",
        ),
    ] {
        let fixture = Fixture::new(&schema);
        assert_eq!(
            fixture.generate().expect_err("ChatType drift").kind(),
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
        .find("fn runtime_signal_families")
        .expect("runtime recognizer start");
    let recognizer_end = capability_source[recognizer_start..]
        .find("fn contains_word_sequence")
        .map(|offset| recognizer_start + offset)
        .expect("runtime recognizer end");
    assert_eq!(
        sha256_hex(&capability_source.as_bytes()[recognizer_start..recognizer_end]),
        "1c928f16f6ebc397cea201960984e37688e983c86b7dfced14f6c399283ba997",
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
    let mut terminal_non_gates = Vec::new();
    let mut unsupported = signaled
        .into_iter()
        .filter_map(|method| match documented_runtime_requirements(method) {
            Ok(Some(_)) => {
                supported.push(method.name());
                None
            }
            Ok(None) => {
                terminal_non_gates.push(method.name());
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
        hash_method_set(supported.clone()),
        (
            52,
            "da693e7ee0f44569abeedc82e0c83e0442893b7b70d67687a40c0a48e3062494".to_owned()
        ),
        "reviewed real runtime-contract set drift"
    );
    assert_eq!(
        hash_method_set(terminal_non_gates.clone()),
        (
            3,
            "93add10667b68f96b5f8005668163b3627d1ed9eface6d7c06c5b5ab414cbdc0".to_owned()
        ),
        "terminal lexical non-gate set drift"
    );
    supported.extend(terminal_non_gates);
    assert_eq!(
        hash_method_set(supported),
        (
            55,
            "95da2d6299f9ca3d9e1b43553f084996bc3106b668ef1385dfc3af20fe3a979f".to_owned()
        ),
        "terminal runtime-disposition set drift"
    );
    unsupported.sort_unstable();
    let mut unsupported_oracle = unsupported.join("\n");
    unsupported_oracle.push('\n');
    assert_eq!(
        unsupported.len(),
        138,
        "reviewed runtime-disposition boundary drift"
    );
    assert_eq!(
        sha256_hex(unsupported_oracle.as_bytes()),
        "a2028d7acb1055b4c5fc5a0fda69cf4a8c09200feea2fd3d386596e24fc9aa67",
        "reviewed runtime-disposition boundary hash drift"
    );
}

#[test]
fn runtime_signal_scanner_preserves_source_and_overlapping_families() {
    assert_eq!(
        runtime_signal_families("requires can_delete_messages right"),
        [
            RuntimeSignalFamily::RequiresRightPhrase,
            RuntimeSignalFamily::NamedRight(ChatAdministratorRight::CanDeleteMessages),
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        runtime_signal_families("messageproperties.can_be_edited"),
        [
            RuntimeSignalFamily::MessagePropertiesFact,
            RuntimeSignalFamily::CanFieldReference,
        ]
        .into_iter()
        .collect()
    );
}

#[test]
fn pinned_runtime_signal_keys_and_dispositions_are_exact() {
    let schema =
        Schema::parse(include_str!("../../../../vendor/tdlib/td_api.tl")).expect("pinned schema");
    let mut keys = Vec::new();
    let mut semantic = Vec::new();
    let mut source_tags = std::collections::BTreeSet::new();
    let mut family_count_by_source = BTreeMap::new();
    let mut method_sources = BTreeMap::<&str, std::collections::BTreeSet<String>>::new();
    for method in schema.methods() {
        let dispositions = documented_runtime_signal_dispositions(method)
            .unwrap_or_else(|error| panic!("{}: {error}", method.name()));
        for (key, disposition) in dispositions {
            let source = match key.source() {
                RuntimeSignalSource::Description => "description".to_owned(),
                RuntimeSignalSource::Argument(argument) => {
                    format!("argument:{}", argument.as_str())
                }
            };
            method_sources
                .entry(method.name())
                .or_default()
                .insert(source.clone());
            source_tags.insert((method.name(), source.clone()));
            *family_count_by_source
                .entry((method.name(), source.clone()))
                .or_insert(0_usize) += 1;
            let family = key.family().canonical_name();
            keys.push(format!("{}\t{source}\t{family}", method.name()));
            semantic.push(format!(
                "{}\t{source}\t{family}\t{}",
                method.name(),
                disposition.canonical_name()
            ));
        }
    }
    keys.sort_unstable();
    semantic.sort_unstable();
    let hash_rows = |rows: Vec<String>| {
        let mut payload = rows.join("\n");
        payload.push('\n');
        sha256_hex(payload.as_bytes())
    };
    assert_eq!(keys.len(), 398);
    assert_eq!(
        hash_rows(keys),
        "b0b95745adac694757ae7a46dcbb4dce048129379c3aefa62da62f04a2476545"
    );
    assert_eq!(
        hash_rows(semantic),
        "f3f2c8c344d4082ac918f4b4a279f3d863db51760dcf1a5074711faef5e25a58"
    );
    assert_eq!(source_tags.len(), 208, "signaled source-tag count");
    assert_eq!(
        source_tags
            .iter()
            .filter(|(_, source)| source == "description")
            .count(),
        162,
        "description source-tag count"
    );
    assert_eq!(
        source_tags
            .iter()
            .filter(|(_, source)| source.starts_with("argument:"))
            .count(),
        46,
        "argument source-tag count"
    );
    assert_eq!(family_count_by_source.values().copied().max(), Some(4));
    assert!(method_sources.values().all(|sources| sources.len() <= 3));

    let retry = schema
        .methods()
        .iter()
        .flat_map(|method| {
            documented_runtime_signal_dispositions(method)
                .expect("signal dispositions")
                .into_iter()
                .filter_map(move |(_, disposition)| {
                    (disposition
                        == RuntimeSignalDisposition::Deferred(DeferredSignalLane::RetryCondition))
                    .then_some(method.name())
                })
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        retry,
        ["readdQuickReplyShortcutMessages", "resendMessages"]
            .into_iter()
            .collect()
    );
}

#[test]
fn parses_schema_bound_chat_kind_atoms() {
    let schema = Schema::parse(SCHEMA).expect("fixture schema");
    let dto: RuntimeRequirementsDto = serde_json::from_value(json!({
        "kind": "any_of",
        "clauses": [{"all_of": [{
            "kind": "chat_kind",
            "target_argument": "chat_id",
            "value": "channel"
        }]}]
    }))
    .expect("closed chat-kind DTO");
    let requirements = parse_runtime_requirements(dto, find_method(&schema, "unpinChatMessage"))
        .expect("schema-bound chat-kind atom");
    assert!(matches!(
        requirements.clauses(),
        [clause]
            if matches!(
                clause.as_slice(),
                [RuntimeRequirement::ChatKind(condition)]
                    if condition.kind() == ResolvedChatKind::Channel
                        && condition.target().argument().as_str() == "chat_id"
            )
    ));
}

#[test]
fn parses_schema_bound_message_capability_atoms() {
    let schema =
        Schema::parse(include_str!("../../../../vendor/tdlib/td_api.tl")).expect("pinned schema");
    let parse = |method: &str, value: Value| {
        let dto: RuntimeRequirementsDto = serde_json::from_value(json!({
            "kind": "any_of",
            "clauses": [{"all_of": [value]}]
        }))
        .expect("runtime requirement DTO");
        parse_runtime_requirements(dto, find_method(&schema, method))
    };

    let one = parse(
        "editMessageText",
        json!({
            "kind": "message_capability",
            "subject": {
                "kind": "one",
                "chat_argument": "chat_id",
                "message_argument": "message_id"
            },
            "capability": "can_be_edited"
        }),
    )
    .expect("scalar message capability");
    assert!(matches!(
        one.clauses(),
        [clause]
            if matches!(
                clause.as_slice(),
                [RuntimeRequirement::MessageCapability { subject, capability }]
                    if *capability == MessageCapability::CanBeEdited
                        && matches!(subject, MessageSubjectRef::One { .. })
            )
    ));

    let each = parse(
        "reportSupergroupSpam",
        json!({
            "kind": "message_capability",
            "subject": {
                "kind": "each",
                "chat_argument": "supergroup_id",
                "message_argument": "message_ids"
            },
            "capability": "can_report_supergroup_spam"
        }),
    )
    .expect("universal message capability");
    assert!(matches!(
        each.clauses(),
        [clause]
            if matches!(
                clause.as_slice(),
                [RuntimeRequirement::MessageCapability { subject, capability }]
                    if *capability == MessageCapability::CanReportSupergroupSpam
                        && matches!(subject, MessageSubjectRef::Each { .. })
            )
    ));
    assert_eq!(
        serde_json::to_value(super::CanonicalRuntimeRequirement::from_domain(
            &one.clauses()[0][0]
        ))
        .expect("canonical scalar message capability"),
        json!({
            "kind": "message_capability",
            "subject": {
                "kind": "one",
                "chat": {"kind": "chat_id", "argument": "chat_id"},
                "message_argument": "message_id"
            },
            "capability": "can_be_edited"
        })
    );
    assert_eq!(
        serde_json::to_value(super::CanonicalRuntimeRequirement::from_domain(
            &each.clauses()[0][0]
        ))
        .expect("canonical universal message capability"),
        json!({
            "kind": "message_capability",
            "subject": {
                "kind": "each",
                "chat": {"kind": "supergroup_id", "argument": "supergroup_id"},
                "message_argument": "message_ids"
            },
            "capability": "can_report_supergroup_spam"
        })
    );

    for (method, subject) in [
        (
            "editMessageText",
            json!({
                "kind": "each",
                "chat_argument": "chat_id",
                "message_argument": "message_id"
            }),
        ),
        (
            "reportSupergroupSpam",
            json!({
                "kind": "one",
                "chat_argument": "supergroup_id",
                "message_argument": "message_ids"
            }),
        ),
    ] {
        assert_eq!(
            parse(
                method,
                json!({
                    "kind": "message_capability",
                    "subject": subject,
                    "capability": "can_be_edited"
                })
            )
            .expect_err("cardinality/type mismatch")
            .kind(),
            CapabilityGenerationErrorKind::InvalidPolicy
        );
    }

    assert_eq!(
        parse(
            "editMessageText",
            json!({
                "kind": "message_capability",
                "subject": {
                    "kind": "one",
                    "chat_argument": "chat_id",
                    "message_argument": "message_id"
                },
                "capability": "can_fly"
            })
        )
        .expect_err("unknown message capability")
        .kind(),
        CapabilityGenerationErrorKind::InvalidPolicy
    );
    assert!(
        serde_json::from_value::<RuntimeRequirementsDto>(json!({
            "kind": "any_of",
            "clauses": [{"all_of": [{
                "kind": "message_capability",
                "subject": {
                    "kind": "one",
                    "chat_argument": "chat_id",
                    "message_argument": "message_id",
                    "unexpected": true
                },
                "capability": "can_be_edited"
            }]}]
        }))
        .is_err(),
        "message subject DTO must reject unknown fields"
    );

    let pinned = include_str!("../../../../vendor/tdlib/td_api.tl");
    for schema in [
        pinned.replace(
            "reportSupergroupSpam supergroup_id:int53 message_ids:vector<int53>",
            "reportSupergroupSpam supergroup_id:int53 message_ids:int53",
        ),
        pinned.replace(
            "reportSupergroupSpam supergroup_id:int53 message_ids:vector<int53>",
            "reportSupergroupSpam supergroup_id:int53 message_ids:vector<int32>",
        ),
        pinned.replace(
            "reportSupergroupSpam supergroup_id:int53 message_ids:vector<int53>",
            "reportSupergroupSpam supergroup_id:int53 message_ids:vector<vector<int53>>",
        ),
    ] {
        let schema = Schema::parse(&schema).expect("valid wrong-shape schema");
        let dto: RuntimeRequirementsDto = serde_json::from_value(json!({
            "kind": "any_of",
            "clauses": [{"all_of": [{
                "kind": "message_capability",
                "subject": {
                    "kind": "each",
                    "chat_argument": "supergroup_id",
                    "message_argument": "message_ids"
                },
                "capability": "can_report_supergroup_spam"
            }]}]
        }))
        .expect("universal requirement DTO");
        assert_eq!(
            parse_runtime_requirements(dto, find_method(&schema, "reportSupergroupSpam"))
                .expect_err("non-exact vector<int53> must fail")
                .kind(),
            CapabilityGenerationErrorKind::InvalidPolicy
        );
    }
}

#[test]
fn parses_schema_bound_group_call_atoms() {
    let schema =
        Schema::parse(include_str!("../../../../vendor/tdlib/td_api.tl")).expect("pinned schema");
    let parse = |method: &str, atoms: Vec<Value>| {
        let dto: RuntimeRequirementsDto = serde_json::from_value(json!({
            "kind": "any_of",
            "clauses": [{"all_of": atoms}]
        }))
        .expect("group-call requirement DTO");
        parse_runtime_requirements(dto, find_method(&schema, method))
    };

    let title = parse(
        "setVideoChatTitle",
        vec![
            json!({
                "kind": "group_call_kind",
                "group_call_argument": "group_call_id",
                "value": "video_chat"
            }),
            json!({
                "kind": "group_call_property",
                "group_call_argument": "group_call_id",
                "property": "can_be_managed"
            }),
        ],
    )
    .expect("typed group-call kind and property");
    assert!(title.clauses()[0].iter().any(|requirement| matches!(
        requirement,
        RuntimeRequirement::GroupCallKind(condition)
            if condition.kind() == ResolvedGroupCallKind::VideoChat
                && condition.group_call().argument().as_str() == "group_call_id"
    )));
    assert!(title.clauses()[0].iter().any(|requirement| matches!(
        requirement,
        RuntimeRequirement::GroupCallProperty { group_call, property }
            if *property == GroupCallProperty::CanBeManaged
                && group_call.argument().as_str() == "group_call_id"
    )));

    let messages = parse(
        "deleteGroupCallMessages",
        vec![json!({
            "kind": "group_call_message_capability",
            "subject": {
                "kind": "each",
                "group_call_argument": "group_call_id",
                "message_argument": "message_ids"
            },
            "capability": "can_be_deleted"
        })],
    )
    .expect("universal group-call message capability");
    assert!(matches!(
        messages.clauses(),
        [clause]
            if matches!(
                clause.as_slice(),
                [RuntimeRequirement::GroupCallMessageCapability { subject, capability }]
                    if *capability == GroupCallMessageCapability::CanBeDeleted
                        && matches!(subject, GroupCallMessageSubjectRef::Each { .. })
            )
    ));
    assert_eq!(
        serde_json::to_value(super::CanonicalRuntimeRequirement::from_domain(
            &messages.clauses()[0][0]
        ))
        .expect("canonical group-call message capability"),
        json!({
            "kind": "group_call_message_capability",
            "subject": {
                "kind": "each",
                "group_call_argument": "group_call_id",
                "message_argument": "message_ids"
            },
            "capability": "can_be_deleted"
        })
    );

    for atom in [
        json!({
            "kind": "group_call_kind",
            "group_call_argument": "group_call_id",
            "value": "conference"
        }),
        json!({
            "kind": "group_call_property",
            "group_call_argument": "group_call_id",
            "property": "is_active"
        }),
    ] {
        assert_eq!(
            parse("setVideoChatTitle", vec![atom])
                .expect_err("unknown group-call vocabulary")
                .kind(),
            CapabilityGenerationErrorKind::InvalidPolicy
        );
    }

    let wrong_vector = include_str!("../../../../vendor/tdlib/td_api.tl").replace(
        "deleteGroupCallMessages group_call_id:int32 message_ids:vector<int32>",
        "deleteGroupCallMessages group_call_id:int32 message_ids:vector<int53>",
    );
    let wrong_vector = Schema::parse(&wrong_vector).expect("valid wrong-vector schema");
    let dto: RuntimeRequirementsDto = serde_json::from_value(json!({
        "kind": "any_of",
        "clauses": [{"all_of": [{
            "kind": "group_call_message_capability",
            "subject": {
                "kind": "each",
                "group_call_argument": "group_call_id",
                "message_argument": "message_ids"
            },
            "capability": "can_be_deleted"
        }]}]
    }))
    .expect("universal group-call message DTO");
    assert_eq!(
        parse_runtime_requirements(dto, find_method(&wrong_vector, "deleteGroupCallMessages"))
            .expect_err("group-call message_ids needs exact vector<int32>")
            .kind(),
        CapabilityGenerationErrorKind::InvalidPolicy
    );
    assert!(
        serde_json::from_value::<RuntimeRequirementsDto>(json!({
            "kind": "any_of",
            "clauses": [{"all_of": [{
                "kind": "group_call_message_capability",
                "subject": {
                    "kind": "each",
                    "group_call_argument": "group_call_id",
                    "message_argument": "message_ids",
                    "unexpected": true
                },
                "capability": "can_be_deleted"
            }]}]
        }))
        .is_err(),
        "group-call message subject DTO must reject unknown fields"
    );
}

#[test]
fn requires_exact_message_properties_schema_vocabulary() {
    let pinned = include_str!("../../../../vendor/tdlib/td_api.tl");
    validate_message_properties_vocabulary(&Schema::parse(pinned).expect("pinned schema"))
        .expect("exact pinned MessageProperties contract");

    let constructor = pinned
        .lines()
        .find(|line| line.starts_with("messageProperties "))
        .expect("messageProperties constructor");
    let method = "getMessageProperties chat_id:int53 message_id:int53 = MessageProperties;";
    let drifted = [
        pinned.replace(
            "messageProperties can_add_offer:Bool can_add_tasks:Bool",
            "messageProperties can_add_tasks:Bool can_add_offer:Bool",
        ),
        pinned.replace("can_set_fact_check:Bool", "can_set_fact_check:int32"),
        pinned.replace(
            " need_show_statistics:Bool = MessageProperties;",
            " = MessageProperties;",
        ),
        pinned.replace(
            "messageProperties can_add_offer:Bool",
            "messagePropertiesOther can_add_offer:Bool",
        ),
        pinned.replace(
            "getMessageProperties chat_id:int53 message_id:int53 = MessageProperties;",
            "getMessageProperties chat_id:int53 message_id:int32 = MessageProperties;",
        ),
        pinned.replace("can_add_offer:Bool", "can_add_offer_other:Bool"),
        pinned.replace(
            " need_show_statistics:Bool = MessageProperties;",
            " need_show_statistics:Bool extra:Bool = MessageProperties;",
        ),
    ];
    assert!(
        Schema::parse(&pinned.replace(
            " need_show_statistics:Bool = MessageProperties;",
            " need_show_statistics:Bool = MessageProperties<int53>;",
        ))
        .is_err(),
        "generic MessageProperties result must fail at the schema boundary"
    );
    let duplicate_constructor = pinned.replace(
        "---functions---",
        &format!("{constructor}\n\n---functions---"),
    );
    assert!(
        Schema::parse(&duplicate_constructor).is_err(),
        "duplicate MessageProperties constructor must fail at the schema boundary"
    );
    let duplicate_method = pinned.replacen(method, &format!("{method}\n{method}"), 1);
    assert!(
        Schema::parse(&duplicate_method).is_err(),
        "duplicate getMessageProperties method must fail at the schema boundary"
    );

    for schema in drifted {
        let error = validate_message_properties_vocabulary(
            &Schema::parse(&schema).expect("valid drifted schema"),
        )
        .expect_err("MessageProperties drift");
        assert_eq!(error.kind(), CapabilityGenerationErrorKind::SchemaDrift);
    }

    let group_call = pinned
        .lines()
        .find(|line| line.starts_with("groupCall "))
        .expect("groupCall constructor");
    let extra_constructor = group_call.replacen("groupCall ", "groupCallOther ", 1);
    let extra_constructor = pinned.replace(
        "---functions---",
        &format!("{extra_constructor}\n\n---functions---"),
    );
    assert_eq!(
        validate_group_call_vocabulary(
            &Schema::parse(&extra_constructor).expect("second GroupCall constructor")
        )
        .expect_err("extra constructor with GroupCall result")
        .kind(),
        CapabilityGenerationErrorKind::SchemaDrift
    );

    for signature in [
        "getGroupCall group_call_id:int32 = GroupCall;",
        "updateNewGroupCallMessage group_call_id:int32 message:groupCallMessage = Update;",
    ] {
        assert!(
            Schema::parse(&pinned.replacen(signature, &format!("{signature}\n{signature}"), 1))
                .is_err(),
            "duplicate exact definition must fail at the schema boundary"
        );
    }
}

#[test]
fn requires_exact_group_call_schema_vocabulary() {
    let pinned = include_str!("../../../../vendor/tdlib/td_api.tl");
    validate_group_call_vocabulary(&Schema::parse(pinned).expect("pinned schema"))
        .expect("exact pinned GroupCall contract");

    let drifted = [
        pinned.replace(
            "groupCall id:int32 unique_id:int64",
            "groupCall unique_id:int64 id:int32",
        ),
        pinned.replace(" can_be_managed:Bool", " can_be_managed:int32"),
        pinned.replace(" can_delete_messages:Bool", " can_delete_messages_other:Bool"),
        pinned.replace(
            "groupCallMessage message_id:int32 sender_id:MessageSender",
            "groupCallMessage sender_id:MessageSender message_id:int32",
        ),
        pinned.replace(" can_be_deleted:Bool = GroupCallMessage;", " = GroupCallMessage;"),
        pinned.replace(
            "getGroupCall group_call_id:int32 = GroupCall;",
            "getGroupCall group_call_id:int53 = GroupCall;",
        ),
        pinned.replace(
            "updateNewGroupCallMessage group_call_id:int32 message:groupCallMessage = Update;",
            "updateNewGroupCallMessage group_call_id:int32 message:GroupCallMessage = Update;",
        ),
        pinned.replace(
            "updateGroupCall group_call:groupCall = Update;",
            "updateGroupCall group_call:GroupCall = Update;",
        ),
        pinned.replace(
            "updateGroupCallMessagesDeleted group_call_id:int32 message_ids:vector<int32> = Update;",
            "updateGroupCallMessagesDeleted group_call_id:int32 message_ids:vector<int53> = Update;",
        ),
    ];
    for schema in drifted {
        let error = validate_group_call_vocabulary(
            &Schema::parse(&schema).expect("valid drifted group-call schema"),
        )
        .expect_err("GroupCall schema drift");
        assert_eq!(error.kind(), CapabilityGenerationErrorKind::SchemaDrift);
    }

    for signature in [
        "updateGroupCall group_call:groupCall = Update;",
        "updateNewGroupCallMessage group_call_id:int32 message:groupCallMessage = Update;",
        "updateGroupCallMessagesDeleted group_call_id:int32 message_ids:vector<int32> = Update;",
    ] {
        let without_update = pinned.replacen(signature, "", 1);
        assert_ne!(without_update, pinned, "update movement fixture");
        let moved_to_methods = without_update.replacen(
            "---functions---",
            &format!("---functions---\n{signature}"),
            1,
        );
        let moved_to_methods =
            Schema::parse(&moved_to_methods).expect("valid update-to-method drift schema");
        assert_eq!(
            validate_group_call_vocabulary(&moved_to_methods)
                .expect_err("update ingress must remain a constructor")
                .kind(),
            CapabilityGenerationErrorKind::SchemaDrift,
            "{signature}"
        );
    }
}

#[test]
fn requires_exact_supergroup_full_info_schema_vocabulary() {
    let pinned = include_str!("../../../../vendor/tdlib/td_api.tl");
    validate_supergroup_full_info_vocabulary(&Schema::parse(pinned).expect("pinned schema"))
        .expect("exact pinned SupergroupFullInfo contract");

    let drifted = [
        pinned.replace(
            "supergroupFullInfo photo:chatPhoto community_id:int53",
            "supergroupFullInfo community_id:int53 photo:chatPhoto",
        ),
        pinned.replace(" can_get_members:Bool", " can_get_members:int32"),
        pinned.replace(" can_hide_members:Bool", " can_hide_participants:Bool"),
        pinned.replace(
            " upgraded_from_max_message_id:int53 = SupergroupFullInfo;",
            " = SupergroupFullInfo;",
        ),
        pinned.replace(
            " upgraded_from_max_message_id:int53 = SupergroupFullInfo;",
            " upgraded_from_max_message_id:int53 extra:Bool = SupergroupFullInfo;",
        ),
        pinned.replace(
            "getSupergroupFullInfo supergroup_id:int53 = SupergroupFullInfo;",
            "getSupergroupFullInfo supergroup_id:int32 = SupergroupFullInfo;",
        ),
        pinned.replace(
            "updateSupergroupFullInfo supergroup_id:int53 supergroup_full_info:supergroupFullInfo = Update;",
            "updateSupergroupFullInfo supergroup_id:int32 supergroup_full_info:supergroupFullInfo = Update;",
        ),
    ];
    for schema in drifted {
        let error = validate_supergroup_full_info_vocabulary(
            &Schema::parse(&schema).expect("valid drifted full-info schema"),
        )
        .expect_err("SupergroupFullInfo schema drift");
        assert_eq!(error.kind(), CapabilityGenerationErrorKind::SchemaDrift);
    }

    let signature = "updateSupergroupFullInfo supergroup_id:int53 supergroup_full_info:supergroupFullInfo = Update;";
    let without_update = pinned.replacen(signature, "", 1);
    let moved_to_methods = without_update.replacen(
        "---functions---",
        &format!("---functions---\n{signature}"),
        1,
    );
    let moved_to_methods =
        Schema::parse(&moved_to_methods).expect("valid update-to-method drift schema");
    assert_eq!(
        validate_supergroup_full_info_vocabulary(&moved_to_methods)
            .expect_err("full-info ingress must remain an update constructor")
            .kind(),
        CapabilityGenerationErrorKind::SchemaDrift
    );
}

#[test]
fn pinned_message_capability_contracts_are_exact() {
    let pinned = include_str!("../../../../vendor/tdlib/td_api.tl");
    let schema = Schema::parse(pinned).expect("pinned schema");
    let safe_methods = [
        "addChecklistTasks",
        "addOffer",
        "approveSuggestedPost",
        "declineSuggestedPost",
        "deleteMessageReactionsFromSender",
        "editMessageCaption",
        "editMessageChecklist",
        "editMessageLiveLocation",
        "editMessageMedia",
        "editMessageReplyMarkup",
        "editMessageSchedulingState",
        "editMessageText",
        "getMessageAuthor",
        "getMessageEmbeddingCode",
        "getMessagePublicForwards",
        "getMessageReadDate",
        "getMessageStatistics",
        "getMessageThread",
        "getMessageThreadHistory",
        "getMessageViewers",
        "getPollVoteStatistics",
        "getVideoMessageAdvertisements",
        "markChecklistTasksAsDone",
        "pinChatMessage",
        "recognizeSpeech",
        "reportMessageReactions",
        "reportSupergroupSpam",
        "setMessageFactCheck",
        "stopPoll",
    ];
    let unsafe_methods = [
        "deleteMessages",
        "forwardMessages",
        "getMessageLink",
        "reportChat",
    ];
    let hash_rows = |mut rows: Vec<String>| {
        rows.sort_unstable();
        let mut payload = rows.join("\n");
        payload.push('\n');
        sha256_hex(payload.as_bytes())
    };
    let safe_set = safe_methods
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let unsafe_set = unsafe_methods
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    assert!(safe_set.is_disjoint(&unsafe_set));
    let reviewed_partition = safe_set
        .union(&unsafe_set)
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let derived_message_property_methods = schema
        .methods()
        .iter()
        .filter_map(|method| {
            documented_runtime_signal_dispositions(method)
                .unwrap_or_else(|error| panic!("{}: {error}", method.name()))
                .iter()
                .any(|(key, _)| key.family() == RuntimeSignalFamily::MessagePropertiesFact)
                .then_some(method.name())
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        reviewed_partition, derived_message_property_methods,
        "safe/deferred MessageProperties partition must be exhaustive"
    );
    assert_eq!(reviewed_partition.len(), 33);
    assert_eq!(
        hash_rows(reviewed_partition.iter().map(ToString::to_string).collect()),
        "5f04f36c0e2862498474a4a1651d2f5131f3adb80d6cef766b33c9c2bf11e8fc",
        "schema-derived MessageProperties method set drift"
    );
    assert_eq!(
        hash_rows(safe_methods.iter().map(ToString::to_string).collect()),
        "45d98c8243fd32ac9b9fc0234b73a2946e1eb62813cbac30cabaa043c7e53cba",
        "reviewed message-capability method set drift"
    );

    let mut rows = Vec::new();
    let mut consumed_key_rows = Vec::new();
    for method_name in safe_methods {
        let method = find_method(&schema, method_name);
        let dispositions = documented_runtime_signal_dispositions(method)
            .unwrap_or_else(|error| panic!("{method_name}: {error}"));
        let sources = dispositions
            .iter()
            .filter(|(key, disposition)| {
                *disposition == RuntimeSignalDisposition::ConsumedByRuntimeRequirements
                    && key.family() == RuntimeSignalFamily::MessagePropertiesFact
            })
            .map(|(key, _)| match key.source() {
                RuntimeSignalSource::Description => "description".to_owned(),
                RuntimeSignalSource::Argument(argument) => {
                    format!("argument:{}", argument.as_str())
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(sources.len(), 1, "{method_name}: exact property source");
        consumed_key_rows.extend(
            dispositions
                .iter()
                .filter(|(_, disposition)| {
                    *disposition == RuntimeSignalDisposition::ConsumedByRuntimeRequirements
                })
                .map(|(key, _)| {
                    let source = match key.source() {
                        RuntimeSignalSource::Description => "description".to_owned(),
                        RuntimeSignalSource::Argument(argument) => {
                            format!("argument:{}", argument.as_str())
                        }
                    };
                    format!("{method_name}\t{source}\t{}", key.family().canonical_name())
                }),
        );

        let requirements = documented_runtime_requirements(method)
            .unwrap_or_else(|error| panic!("{method_name}: {error}"))
            .expect("reviewed message capability contract");
        match method_name {
            "addOffer" => assert_eq!(requirements.clauses().len(), 2),
            _ => assert_eq!(requirements.clauses().len(), 1, "{method_name}"),
        }
        if method_name == "reportSupergroupSpam" {
            let clause = &requirements.clauses()[0];
            assert!(clause.iter().any(|atom| matches!(
                atom,
                RuntimeRequirement::ChatKind(condition)
                    if condition.kind() == ResolvedChatKind::Supergroup
                        && condition.target().argument().as_str() == "supergroup_id"
            )));
            assert!(clause.iter().any(|atom| matches!(
                atom,
                RuntimeRequirement::ChatAdministrator { target }
                    if target.argument().as_str() == "supergroup_id"
            )));
        }
        for clause in requirements.clauses() {
            let mut property_atoms = 0;
            for atom in clause {
                if let RuntimeRequirement::MessageCapability {
                    subject,
                    capability,
                } = atom
                {
                    property_atoms += 1;
                    let (cardinality, chat, message) = match subject {
                        MessageSubjectRef::One { chat, message } => {
                            ("one", chat.argument().as_str(), message.argument().as_str())
                        }
                        MessageSubjectRef::Each { chat, messages } => (
                            "all",
                            chat.argument().as_str(),
                            messages.argument().as_str(),
                        ),
                    };
                    rows.push(format!(
                        "{method_name}\t{}\t{chat}\t{message}\t{cardinality}\t{}",
                        sources[0],
                        capability.as_str()
                    ));
                }
            }
            assert_eq!(
                property_atoms, 1,
                "{method_name}: one property atom per clause"
            );
            let expected_atoms = if method_name == "reportSupergroupSpam" {
                3
            } else {
                1
            };
            assert_eq!(clause.len(), expected_atoms, "{method_name}: exact clause");
        }
    }
    assert_eq!(rows.len(), 30, "exact message-property binding count");
    assert_eq!(
        hash_rows(rows),
        "fee0c5dc03c67084e46b0d20a8158dcecbf38c58676a149fe9140d8469aeb50b",
        "message-property binding oracle drift"
    );
    assert_eq!(
        consumed_key_rows.len(),
        59,
        "exact consumed signal-key count"
    );
    assert_eq!(
        hash_rows(consumed_key_rows),
        "c4f91a61456297edd4a9a2fe206d3d37410cc2eb67f6f73e9661985949175ed2",
        "consumed message-contract signal-key oracle drift"
    );

    assert_eq!(
        hash_rows(unsafe_methods.iter().map(ToString::to_string).collect()),
        "a7755c8b6787c2ea596a45f6a17a4af970a735382721ee949aec54beb8602317",
        "deferred mixed-semantics method set drift"
    );
    let mut deferred_key_rows = Vec::new();
    for method_name in unsafe_methods {
        let error = documented_runtime_requirements(find_method(&schema, method_name))
            .expect_err("mixed message-property semantics must remain open");
        assert_eq!(error.kind(), CapabilityGenerationErrorKind::SchemaDrift);
        deferred_key_rows.extend(
            documented_runtime_signal_dispositions(find_method(&schema, method_name))
                .expect("deferred mixed-method dispositions")
                .into_iter()
                .map(|(key, disposition)| {
                    assert!(matches!(disposition, RuntimeSignalDisposition::Deferred(_)));
                    let source = match key.source() {
                        RuntimeSignalSource::Description => "description".to_owned(),
                        RuntimeSignalSource::Argument(argument) => {
                            format!("argument:{}", argument.as_str())
                        }
                    };
                    format!("{method_name}\t{source}\t{}", key.family().canonical_name())
                }),
        );
    }
    assert_eq!(
        deferred_key_rows.len(),
        11,
        "exact deferred mixed-key count"
    );
    assert_eq!(
        hash_rows(deferred_key_rows),
        "c15898d88f92f2bde554308208952d52fd0add6d33bb0f72635b6959ec7beffc",
        "deferred mixed-method signal-key oracle drift"
    );

    let source = "Identifier of the message. Use messageProperties.can_be_edited to check whether the message can be edited";
    for (name, replacement) in [
        ("missing source", "Identifier of the message"),
        (
            "same field with changed wording",
            "Identifier of the message. Check messageProperties.can_be_edited before editing the message",
        ),
        (
            "different valid field",
            "Identifier of the message. Use messageProperties.can_be_pinned to check whether the message can be edited",
        ),
        (
            "additional valid field",
            "Identifier of the message. Use messageProperties.can_be_edited and messageProperties.can_be_pinned to check whether the message can be edited",
        ),
    ] {
        let mutated = pinned.replacen(source, replacement, 1);
        assert_ne!(mutated, pinned, "{name}: mutation fixture");
        let mutated = Schema::parse(&mutated).expect("valid source mutation");
        let error = documented_runtime_requirements(find_method(&mutated, "editMessageText"))
            .expect_err("message-property source drift must fail closed");
        assert_eq!(
            error.kind(),
            CapabilityGenerationErrorKind::SchemaDrift,
            "{name}"
        );
    }

    let duplicate_source = pinned.replacen(
        source,
        &format!("{source}\n//@message_id Duplicate non-gating documentation"),
        1,
    );
    let duplicate_source = Schema::parse(&duplicate_source).expect("duplicate source-tag schema");
    let error = documented_runtime_requirements(find_method(&duplicate_source, "editMessageText"))
        .expect_err("duplicate reviewed source tag must fail closed");
    assert_eq!(error.kind(), CapabilityGenerationErrorKind::SchemaDrift);

    let wrong_scalar = pinned.replace(
        "editMessageText chat_id:int53 message_id:int53",
        "editMessageText chat_id:int53 message_id:int32",
    );
    let wrong_scalar = Schema::parse(&wrong_scalar).expect("valid scalar-type mutation");
    let error = documented_runtime_requirements(find_method(&wrong_scalar, "editMessageText"))
        .expect_err("scalar message_id type drift must fail closed");
    assert_eq!(error.kind(), CapabilityGenerationErrorKind::SchemaDrift);

    for (name, mutated) in [
        (
            "administrator wording",
            pinned.replace(
                "Reports messages in a supergroup as spam; requires administrator rights in the supergroup",
                "Reports messages in a supergroup as spam; requires administrator access in the supergroup",
            ),
        ),
        (
            "message vector element",
            pinned.replace(
                "reportSupergroupSpam supergroup_id:int53 message_ids:vector<int53>",
                "reportSupergroupSpam supergroup_id:int53 message_ids:vector<int32>",
            ),
        ),
        (
            "chat identifier space",
            pinned.replace(
                "reportSupergroupSpam supergroup_id:int53 message_ids:vector<int53>",
                "reportSupergroupSpam chat_id:int53 message_ids:vector<int53>",
            ),
        ),
    ] {
        let mutated = Schema::parse(&mutated).expect("valid spam-contract mutation");
        let error = documented_runtime_requirements(find_method(&mutated, "reportSupergroupSpam"))
            .expect_err("spam-contract drift must fail closed");
        assert_eq!(
            error.kind(),
            CapabilityGenerationErrorKind::SchemaDrift,
            "{name}"
        );
    }
}

#[test]
fn pinned_group_call_capability_contracts_are_exact() {
    let pinned = include_str!("../../../../vendor/tdlib/td_api.tl");
    let schema = Schema::parse(pinned).expect("pinned schema");
    let safe_methods = [
        "banGroupCallParticipants",
        "deleteGroupCallMessages",
        "deleteGroupCallMessagesBySender",
        "endGroupCall",
        "endGroupCallRecording",
        "revokeGroupCallInviteLink",
        "sendGroupCallMessage",
        "setGroupCallPaidMessageStarCount",
        "setVideoChatTitle",
        "startGroupCallRecording",
        "toggleGroupCallAreMessagesAllowed",
        "toggleVideoChatMuteNewParticipants",
    ];
    let deferred_methods = [
        "getVideoChatInviteLink",
        "toggleGroupCallParticipantIsHandRaised",
    ];
    let hash_rows = |mut rows: Vec<String>| {
        rows.sort_unstable();
        let mut payload = rows.join("\n");
        payload.push('\n');
        sha256_hex(payload.as_bytes())
    };
    let safe = safe_methods
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let deferred = deferred_methods
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    assert!(safe.is_disjoint(&deferred));
    let reviewed = safe
        .union(&deferred)
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let derived = schema
        .methods()
        .iter()
        .filter_map(|method| {
            documented_runtime_signal_dispositions(method)
                .unwrap_or_else(|error| panic!("{}: {error}", method.name()))
                .iter()
                .any(|(key, _)| {
                    matches!(
                        key.family(),
                        RuntimeSignalFamily::GroupCallFact
                            | RuntimeSignalFamily::GroupCallMessageFact
                    )
                })
                .then_some(method.name())
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(reviewed, derived, "group-call partition must be exhaustive");
    assert_eq!(
        hash_rows(reviewed.iter().map(ToString::to_string).collect()),
        "19f031588f1a95638917e614017be240ae1bcb7a139d1c9b8e74c0882b79c2e5"
    );
    assert_eq!(
        hash_rows(safe.iter().map(ToString::to_string).collect()),
        "a889f96632f1e7e61cb292ec5bb97fc6eaf260ce524f2742636216b2fd7c9570"
    );
    assert_eq!(
        hash_rows(deferred.iter().map(ToString::to_string).collect()),
        "08e959dd01394969d60f64a26448a2baf959b82d1deb7b775aeb32b35b336d3e"
    );

    let expected_formulas = std::collections::BTreeMap::from([
        ("banGroupCallParticipants", "kind:unbound&property:is_owned"),
        (
            "deleteGroupCallMessages",
            "kind:live_story&messages:each:can_be_deleted",
        ),
        (
            "deleteGroupCallMessagesBySender",
            "kind:live_story&property:can_delete_messages",
        ),
        (
            "endGroupCall",
            "kind:live_story&property:can_be_managed|kind:unbound&property:is_owned|kind:video_chat&property:can_be_managed",
        ),
        (
            "endGroupCallRecording",
            "kind:video_chat&property:can_be_managed",
        ),
        (
            "revokeGroupCallInviteLink",
            "kind:unbound&property:is_owned|kind:video_chat&property:can_be_managed",
        ),
        ("sendGroupCallMessage", "property:can_send_messages"),
        (
            "setGroupCallPaidMessageStarCount",
            "kind:live_story&property:can_be_managed",
        ),
        (
            "setVideoChatTitle",
            "kind:video_chat&property:can_be_managed",
        ),
        (
            "startGroupCallRecording",
            "kind:video_chat&property:can_be_managed",
        ),
        (
            "toggleGroupCallAreMessagesAllowed",
            "property:can_toggle_are_messages_allowed",
        ),
        (
            "toggleVideoChatMuteNewParticipants",
            "kind:video_chat&property:can_toggle_mute_new_participants",
        ),
    ]);
    let mut consumed_rows = Vec::new();
    for method_name in safe_methods {
        let method = find_method(&schema, method_name);
        let dispositions = documented_runtime_signal_dispositions(method)
            .unwrap_or_else(|error| panic!("{method_name}: {error}"));
        consumed_rows.extend(dispositions.iter().filter_map(|(key, disposition)| {
            let describes_setting_value = method_name == "toggleVideoChatMuteNewParticipants"
                && key.source() == &RuntimeSignalSource::Description
                && key.family() == RuntimeSignalFamily::OnlyByAdministrator;
            if describes_setting_value {
                assert_eq!(
                    *disposition,
                    RuntimeSignalDisposition::NotRuntimeGate(
                        NonGateReason::GroupCallParticipantUnmutePolicy
                    ),
                    "{method_name}: administrator wording describes the configured value"
                );
                return None;
            }
            assert_eq!(
                *disposition,
                RuntimeSignalDisposition::ConsumedByRuntimeRequirements,
                "{method_name}: every exact signal key must be consumed"
            );
            let source = match key.source() {
                RuntimeSignalSource::Description => "description".to_owned(),
                RuntimeSignalSource::Argument(argument) => {
                    format!("argument:{}", argument.as_str())
                }
            };
            Some(format!(
                "{method_name}\t{source}\t{}",
                key.family().canonical_name()
            ))
        }));

        let requirements = documented_runtime_requirements(method)
            .unwrap_or_else(|error| panic!("{method_name}: {error}"))
            .expect("reviewed group-call contract");
        let mut clauses = requirements
            .clauses()
            .iter()
            .map(|clause| {
                let mut atoms = clause
                    .iter()
                    .map(|requirement| match requirement {
                        RuntimeRequirement::GroupCallKind(condition) => {
                            assert_eq!(condition.group_call().argument().as_str(), "group_call_id");
                            format!("kind:{}", condition.kind().as_str())
                        }
                        RuntimeRequirement::GroupCallProperty {
                            group_call,
                            property,
                        } => {
                            assert_eq!(group_call.argument().as_str(), "group_call_id");
                            format!("property:{}", property.as_str())
                        }
                        RuntimeRequirement::GroupCallMessageCapability {
                            subject:
                                GroupCallMessageSubjectRef::Each {
                                    group_call,
                                    messages,
                                },
                            capability,
                        } => {
                            assert_eq!(group_call.argument().as_str(), "group_call_id");
                            assert_eq!(messages.argument().as_str(), "message_ids");
                            format!("messages:each:{}", capability.as_str())
                        }
                        other => panic!("{method_name}: unexpected atom {other:?}"),
                    })
                    .collect::<Vec<_>>();
                atoms.sort_unstable();
                atoms.join("&")
            })
            .collect::<Vec<_>>();
        clauses.sort_unstable();
        assert_eq!(
            clauses.join("|"),
            expected_formulas[method_name],
            "{method_name}: exact typed DNF"
        );
    }
    assert_eq!(consumed_rows.len(), 38, "exact safe group-call key count");
    assert_eq!(
        hash_rows(consumed_rows),
        "baa12c60379a31fd62a3f030b65ac3e87f0827793c340bb7f63f7bff000f1df5"
    );

    let mut deferred_rows = Vec::new();
    for method_name in deferred_methods {
        let method = find_method(&schema, method_name);
        let error = documented_runtime_requirements(method)
            .expect_err("argument-dependent group-call method must stay open");
        assert_eq!(error.kind(), CapabilityGenerationErrorKind::SchemaDrift);
        deferred_rows.extend(
            documented_runtime_signal_dispositions(method)
                .expect("deferred group-call dispositions")
                .into_iter()
                .map(|(key, disposition)| {
                    assert_eq!(
                        disposition,
                        RuntimeSignalDisposition::Deferred(DeferredSignalLane::InputPrerequisite)
                    );
                    let RuntimeSignalSource::Argument(argument) = key.source() else {
                        panic!("{method_name}: deferred source must be an argument")
                    };
                    format!(
                        "{method_name}\targument:{}\t{}",
                        argument.as_str(),
                        key.family().canonical_name()
                    )
                }),
        );
    }
    assert_eq!(deferred_rows.len(), 6);
    assert_eq!(
        hash_rows(deferred_rows),
        "c7d82927a49fb17def723966a8b964c8d4a725fdc755b1c72b97cf59fd5878ef"
    );

    let source = "Sets title of a video chat; requires groupCall.can_be_managed right";
    for replacement in [
        "Sets title of a video chat",
        "Sets title of a video chat; requires groupCall.can_send_messages right",
        "Sets title of an active video chat; requires groupCall.can_be_managed right",
    ] {
        let mutated = pinned.replacen(source, replacement, 1);
        assert_ne!(mutated, pinned, "source mutation fixture");
        let mutated = Schema::parse(&mutated).expect("valid source mutation");
        let error = documented_runtime_requirements(find_method(&mutated, "setVideoChatTitle"))
            .expect_err("group-call source drift must fail closed");
        assert_eq!(error.kind(), CapabilityGenerationErrorKind::SchemaDrift);
    }
    let duplicate_source = pinned.replacen(
        source,
        &format!("{source}\n//@description Duplicate non-gating documentation"),
        1,
    );
    let duplicate_source = Schema::parse(&duplicate_source).expect("duplicate source-tag schema");
    assert_eq!(
        documented_runtime_requirements(find_method(&duplicate_source, "setVideoChatTitle"))
            .expect_err("duplicate reviewed source must fail closed")
            .kind(),
        CapabilityGenerationErrorKind::SchemaDrift
    );
    let additional_signal = pinned.replacen(
        "New group call title; 1-64 characters",
        "New group call title; 1-64 characters. Requires groupCall.can_send_messages right",
        1,
    );
    let additional_signal =
        Schema::parse(&additional_signal).expect("additional argument-signal schema");
    assert_eq!(
        documented_runtime_requirements(find_method(&additional_signal, "setVideoChatTitle"))
            .expect_err("unconsumed additional group-call signal must fail closed")
            .kind(),
        CapabilityGenerationErrorKind::SchemaDrift
    );

    for mutated in [
        pinned.replace(
            "setVideoChatTitle group_call_id:int32",
            "setVideoChatTitle group_call_id:int53",
        ),
        pinned.replace(
            "deleteGroupCallMessages group_call_id:int32 message_ids:vector<int32>",
            "deleteGroupCallMessages group_call_id:int32 message_ids:int32",
        ),
        pinned.replace(
            "deleteGroupCallMessages group_call_id:int32 message_ids:vector<int32>",
            "deleteGroupCallMessages group_call_id:int32 message_ids:vector<int53>",
        ),
        pinned.replace(
            "deleteGroupCallMessages group_call_id:int32 message_ids:vector<int32>",
            "deleteGroupCallMessages group_call_id:int32 message_ids:vector<vector<int32>>",
        ),
    ] {
        let mutated = Schema::parse(&mutated).expect("valid role-shape mutation");
        let method = if mutated.methods().iter().any(|method| {
            method.name() == "setVideoChatTitle"
                && field_type(method, "group_call_id").is_some_and(|ty| ty.name() == "int53")
        }) {
            "setVideoChatTitle"
        } else {
            "deleteGroupCallMessages"
        };
        assert_eq!(
            documented_runtime_requirements(find_method(&mutated, method))
                .expect_err("group-call identifier/cardinality drift must fail closed")
                .kind(),
            CapabilityGenerationErrorKind::SchemaDrift
        );
    }
}

#[test]
fn parses_schema_bound_supergroup_full_info_property_atom() {
    let schema =
        Schema::parse(include_str!("../../../../vendor/tdlib/td_api.tl")).expect("pinned schema");
    let parse = |target_argument: &str, property: &str| {
        let dto: RuntimeRequirementsDto = serde_json::from_value(json!({
            "kind": "any_of",
            "clauses": [{"all_of": [{
                "kind": "supergroup_full_info_property",
                "target_argument": target_argument,
                "property": property
            }]}]
        }))
        .expect("full-info requirement DTO");
        parse_runtime_requirements(dto, find_method(&schema, "getChatStatistics"))
    };

    let requirements =
        parse("chat_id", "can_get_statistics").expect("typed supergroup-full-info property");
    let [RuntimeRequirement::SupergroupFullInfoProperty { target, property }] =
        requirements.clauses()[0].as_slice()
    else {
        panic!("unexpected full-info requirement")
    };
    assert_eq!(target.kind(), ChatTargetKind::ChatId);
    assert_eq!(target.argument().as_str(), "chat_id");
    assert_eq!(*property, SupergroupFullInfoProperty::CanGetStatistics);
    assert_eq!(
        serde_json::to_value(super::CanonicalRuntimeRequirement::from_domain(
            &requirements.clauses()[0][0]
        ))
        .expect("canonical full-info property"),
        json!({
            "kind": "supergroup_full_info_property",
            "target": {"kind": "chat_id", "argument": "chat_id"},
            "property": "can_get_statistics"
        })
    );

    for (target_argument, property) in [
        ("chat_id", "can_send_gift"),
        ("peer_id", "can_get_statistics"),
        ("is_dark", "can_get_statistics"),
    ] {
        assert_eq!(
            parse(target_argument, property)
                .expect_err("unknown property or non-chat target must fail")
                .kind(),
            CapabilityGenerationErrorKind::InvalidPolicy
        );
    }
    assert!(
        serde_json::from_value::<RuntimeRequirementsDto>(json!({
            "kind": "any_of",
            "clauses": [{"all_of": [{
                "kind": "supergroup_full_info_property",
                "target_argument": "chat_id",
                "property": "can_get_statistics",
                "unexpected": true
            }]}]
        }))
        .is_err(),
        "full-info DTO must reject unknown fields"
    );
}

#[test]
fn pinned_supergroup_full_info_capability_contracts_are_exact() {
    let pinned = include_str!("../../../../vendor/tdlib/td_api.tl");
    let schema = Schema::parse(pinned).expect("pinned schema");
    assert_eq!(SupergroupFullInfoProperty::ALL.len(), 8);
    let safe_methods = [
        "getChatStatistics",
        "setChatLocation",
        "setChatPaidMessageStarCount",
        "toggleSupergroupHasAggressiveAntiSpamEnabled",
        "toggleSupergroupHasHiddenMembers",
    ];
    let deferred_methods = [
        "getChatRevenueStatistics",
        "getChatRevenueTransactions",
        "getChatRevenueWithdrawalUrl",
        "getStarRevenueStatistics",
        "getStarTransactions",
        "getSupergroupMembers",
        "setChatDirectMessagesGroup",
    ];
    let hash_rows = |mut rows: Vec<String>| {
        rows.sort_unstable();
        let mut payload = rows.join("\n");
        payload.push('\n');
        sha256_hex(payload.as_bytes())
    };
    let safe = safe_methods
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let deferred = deferred_methods
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    assert!(safe.is_disjoint(&deferred));
    let reviewed = safe
        .union(&deferred)
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let derived = schema
        .methods()
        .iter()
        .filter_map(|method| {
            documented_runtime_signal_dispositions(method)
                .unwrap_or_else(|error| panic!("{}: {error}", method.name()))
                .iter()
                .any(|(key, _)| key.family() == RuntimeSignalFamily::SupergroupFullInfoFact)
                .then_some(method.name())
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(reviewed, derived, "full-info partition must be exhaustive");
    assert_eq!(
        hash_rows(reviewed.iter().map(ToString::to_string).collect()),
        "009753dda10c34a5efd8a8b210f0c2f7addcb51a66d1a9cbcfae1d89f0b85e77"
    );
    assert_eq!(
        hash_rows(safe.iter().map(ToString::to_string).collect()),
        "e2f70c9e185de98cd0e9116f9e63713fbb5dfe50b5dd13ca237eb86e315b4910"
    );
    assert_eq!(
        hash_rows(deferred.iter().map(ToString::to_string).collect()),
        "bf00644532ef19d5546ee88dc805b3c1746867941f7758d613b872e53e63ff47"
    );

    let expected_formulas = std::collections::BTreeMap::from([
        ("getChatStatistics", "full_info:chat_id:can_get_statistics"),
        ("setChatLocation", "full_info:chat_id:can_set_location"),
        (
            "setChatPaidMessageStarCount",
            "administrator_right:chat_id:can_restrict_members&full_info:chat_id:can_enable_paid_messages",
        ),
        (
            "toggleSupergroupHasAggressiveAntiSpamEnabled",
            "full_info:supergroup_id:can_toggle_aggressive_anti_spam",
        ),
        (
            "toggleSupergroupHasHiddenMembers",
            "full_info:supergroup_id:can_hide_members",
        ),
    ]);
    let mut consumed_rows = Vec::new();
    let mut non_gate_rows = Vec::new();
    for method_name in safe_methods {
        let method = find_method(&schema, method_name);
        let dispositions = documented_runtime_signal_dispositions(method)
            .unwrap_or_else(|error| panic!("{method_name}: {error}"));
        for (key, disposition) in &dispositions {
            let source = match key.source() {
                RuntimeSignalSource::Description => "description".to_owned(),
                RuntimeSignalSource::Argument(argument) => {
                    format!("argument:{}", argument.as_str())
                }
            };
            let row = format!("{method_name}\t{source}\t{}", key.family().canonical_name());
            if method_name == "toggleSupergroupHasHiddenMembers"
                && key.family() == RuntimeSignalFamily::OnlyIfAdministrator
            {
                assert!(matches!(
                    disposition,
                    RuntimeSignalDisposition::NotRuntimeGate(
                        NonGateReason::SupergroupFullInfoCrossTokenWording
                    )
                ));
                non_gate_rows.push(row);
            } else {
                assert_eq!(
                    *disposition,
                    RuntimeSignalDisposition::ConsumedByRuntimeRequirements,
                    "{row}"
                );
                consumed_rows.push(row);
            }
        }

        let requirements = documented_runtime_requirements(method)
            .unwrap_or_else(|error| panic!("{method_name}: {error}"))
            .expect("reviewed full-info contract");
        let mut clauses = requirements
            .clauses()
            .iter()
            .map(|clause| {
                let mut atoms = clause
                    .iter()
                    .map(|requirement| match requirement {
                        RuntimeRequirement::SupergroupFullInfoProperty { target, property } => {
                            format!(
                                "full_info:{}:{}",
                                target.argument().as_str(),
                                property.as_str()
                            )
                        }
                        RuntimeRequirement::ChatAdministratorRight { target, right } => {
                            format!(
                                "administrator_right:{}:{}",
                                target.argument().as_str(),
                                right.as_str()
                            )
                        }
                        other => panic!("{method_name}: unexpected atom {other:?}"),
                    })
                    .collect::<Vec<_>>();
                atoms.sort_unstable();
                atoms.join("&")
            })
            .collect::<Vec<_>>();
        clauses.sort_unstable();
        assert_eq!(clauses.join("|"), expected_formulas[method_name]);
    }
    assert_eq!(consumed_rows.len(), 12);
    assert_eq!(
        hash_rows(consumed_rows),
        "1d23853fea29ffe0578cbe963615eb3d5a9fb3e39ce1cbf03753bf42ca1f63fd"
    );

    let mut pending_rows = Vec::new();
    for method_name in deferred_methods {
        let method = find_method(&schema, method_name);
        assert_eq!(
            documented_runtime_requirements(method)
                .expect_err("mixed full-info method must remain open")
                .kind(),
            CapabilityGenerationErrorKind::SchemaDrift
        );
        for (key, disposition) in documented_runtime_signal_dispositions(method)
            .unwrap_or_else(|error| panic!("{method_name}: {error}"))
        {
            let source = match key.source() {
                RuntimeSignalSource::Description => "description".to_owned(),
                RuntimeSignalSource::Argument(argument) => {
                    format!("argument:{}", argument.as_str())
                }
            };
            let row = format!("{method_name}\t{source}\t{}", key.family().canonical_name());
            if method_name == "getSupergroupMembers"
                && key.family() == RuntimeSignalFamily::OnlyIfAdministrator
            {
                assert!(matches!(
                    disposition,
                    RuntimeSignalDisposition::NotRuntimeGate(
                        NonGateReason::SupergroupFullInfoCrossTokenWording
                    )
                ));
                non_gate_rows.push(row);
            } else {
                assert!(matches!(disposition, RuntimeSignalDisposition::Deferred(_)));
                pending_rows.push(row);
            }
        }
    }
    assert_eq!(non_gate_rows.len(), 2);
    assert_eq!(
        hash_rows(non_gate_rows),
        "2e071a4014d657bd19c8ac1ee1f2d954eac565fa2214166519d373f92ed948a5"
    );
    assert_eq!(pending_rows.len(), 18);
    assert_eq!(
        hash_rows(pending_rows),
        "f49347a937191c446fed02419c401e22b8cafc545abda87d0f1b5fe516de95db"
    );

    let source = "Returns detailed statistics about a chat. Currently, this method can be used only for supergroups and channels. Can be used only if supergroupFullInfo.can_get_statistics == true";
    for replacement in [
        "Returns detailed statistics about a chat",
        "Returns detailed statistics about a chat. Currently, this method can be used only for supergroups and channels. Can be used only if supergroupFullInfo.can_get_revenue_statistics == true",
        "Returns detailed chat statistics. Currently, this method can be used only for supergroups and channels. Can be used only if supergroupFullInfo.can_get_statistics == true",
    ] {
        let mutated = pinned.replacen(source, replacement, 1);
        assert_ne!(mutated, pinned, "source mutation fixture");
        let mutated = Schema::parse(&mutated).expect("valid source mutation");
        assert_eq!(
            documented_runtime_requirements(find_method(&mutated, "getChatStatistics"))
                .expect_err("full-info source drift must fail closed")
                .kind(),
            CapabilityGenerationErrorKind::SchemaDrift
        );
    }
    let duplicate_source = pinned.replacen(
        source,
        &format!("{source}\n//@description Duplicate non-gating documentation"),
        1,
    );
    let duplicate_source = Schema::parse(&duplicate_source).expect("duplicate source-tag schema");
    assert_eq!(
        documented_runtime_requirements(find_method(&duplicate_source, "getChatStatistics"))
            .expect_err("duplicate reviewed source must fail closed")
            .kind(),
        CapabilityGenerationErrorKind::SchemaDrift
    );
    let statistics_documentation = format!(
        "{source} @chat_id Chat identifier @is_dark Pass true if a dark theme is used by the application"
    );
    let additional_signal = pinned.replacen(
        &statistics_documentation,
        &format!(
            "{source} @chat_id Chat identifier @is_dark Pass true if a dark theme is used by the application. Use supergroupFullInfo.can_get_statistics to validate it"
        ),
        1,
    );
    assert_ne!(additional_signal, pinned, "additional signal fixture");
    let additional_signal =
        Schema::parse(&additional_signal).expect("additional argument-signal schema");
    assert_eq!(
        documented_runtime_requirements(find_method(&additional_signal, "getChatStatistics"))
            .expect_err("unconsumed additional full-info signal must fail closed")
            .kind(),
        CapabilityGenerationErrorKind::SchemaDrift
    );
    for mutated in [
        pinned.replace(
            "getChatStatistics chat_id:int53 is_dark:Bool",
            "getChatStatistics chat_id:int32 is_dark:Bool",
        ),
        pinned.replace(
            "toggleSupergroupHasHiddenMembers supergroup_id:int53",
            "toggleSupergroupHasHiddenMembers supergroup_id:int32",
        ),
    ] {
        let mutated = Schema::parse(&mutated).expect("valid target-shape mutation");
        let method = if field_type(find_method(&mutated, "getChatStatistics"), "chat_id")
            .is_some_and(|ty| ty.name() == "int32")
        {
            "getChatStatistics"
        } else {
            "toggleSupergroupHasHiddenMembers"
        };
        assert_eq!(
            documented_runtime_requirements(find_method(&mutated, method))
                .expect_err("full-info target type drift must fail closed")
                .kind(),
            CapabilityGenerationErrorKind::SchemaDrift
        );
    }

    let members_source = "Returns information about members or banned users in a supergroup or channel. Can be used only if supergroupFullInfo.can_get_members == true; additionally, administrator privileges may be required for some filters";
    let lexical_drift = pinned.replacen(
        members_source,
        "Returns information about members or banned users in a supergroup or channel. Can be used only if supergroupFullInfo.can_get_members == true; additionally, in practice, administrator privileges may be required for some filters",
        1,
    );
    let lexical_drift = Schema::parse(&lexical_drift).expect("lexical drift schema");
    let only_if =
        documented_runtime_signal_dispositions(find_method(&lexical_drift, "getSupergroupMembers"))
            .expect("drifted dispositions")
            .into_iter()
            .find(|(key, _)| key.family() == RuntimeSignalFamily::OnlyIfAdministrator)
            .expect("cross-token family remains present");
    assert_eq!(
        only_if.1,
        RuntimeSignalDisposition::Deferred(DeferredSignalLane::UnclassifiedDescription),
        "non-gate exception must be exact-source-bound"
    );
}

#[test]
fn pinned_conditional_chat_kind_contracts_are_exact() {
    let schema =
        Schema::parse(include_str!("../../../../vendor/tdlib/td_api.tl")).expect("pinned schema");
    let chat_target = ChatTargetRef::try_from("chat_id").expect("chat target");
    let supergroup_target = ChatTargetRef::try_from("supergroup_id").expect("supergroup target");
    let kind = |target: &ChatTargetRef, value| {
        RuntimeRequirement::ChatKind(
            ChatKindCondition::try_new(target.clone(), value).expect("chat kind"),
        )
    };
    let member = |target: &ChatTargetRef, right| RuntimeRequirement::ChatMemberRight {
        target: target.clone(),
        right,
    };
    let administrator =
        |target: &ChatTargetRef, right| RuntimeRequirement::ChatAdministratorRight {
            target: target.clone(),
            right,
        };
    let assert_contract = |method: &str, clauses| {
        let expected = RequirementAlternatives::try_new(clauses).expect("exact contract");
        assert_eq!(
            documented_runtime_requirements(find_method(&schema, method))
                .expect("reviewed pinned documentation")
                .expect("runtime contract"),
            expected,
            "{method}"
        );
    };

    assert_contract(
        "deleteChatMessagesBySender",
        vec![vec![
            kind(&chat_target, ResolvedChatKind::Supergroup),
            administrator(&chat_target, ChatAdministratorRight::CanDeleteMessages),
        ]],
    );
    assert_contract(
        "addChatMember",
        [
            ResolvedChatKind::BasicGroup,
            ResolvedChatKind::Supergroup,
            ResolvedChatKind::Channel,
        ]
        .into_iter()
        .map(|value| {
            vec![
                kind(&chat_target, value),
                member(&chat_target, ChatMemberRight::CanInviteUsers),
            ]
        })
        .collect(),
    );
    assert_contract(
        "upgradeBasicGroupChatToSupergroupChat",
        vec![vec![
            kind(&chat_target, ResolvedChatKind::BasicGroup),
            RuntimeRequirement::ChatOwner {
                target: chat_target.clone(),
            },
        ]],
    );
    assert_contract(
        "setSupergroupStickerSet",
        vec![vec![
            kind(&supergroup_target, ResolvedChatKind::Supergroup),
            administrator(&supergroup_target, ChatAdministratorRight::CanChangeInfo),
        ]],
    );
    let topic = ForumTopicRef::try_from("forum_topic_id").expect("topic target");
    assert_contract(
        "toggleForumTopicIsClosed",
        vec![
            vec![
                kind(&chat_target, ResolvedChatKind::Supergroup),
                administrator(&chat_target, ChatAdministratorRight::CanManageTopics),
            ],
            vec![
                kind(&chat_target, ResolvedChatKind::Supergroup),
                RuntimeRequirement::TopicCreator {
                    target: chat_target.clone(),
                    topic,
                },
            ],
        ],
    );
    assert_contract(
        "unpinChatMessage",
        vec![
            vec![kind(&chat_target, ResolvedChatKind::Private)],
            vec![kind(&chat_target, ResolvedChatKind::Secret)],
            vec![
                kind(&chat_target, ResolvedChatKind::BasicGroup),
                member(&chat_target, ChatMemberRight::CanPinMessages),
            ],
            vec![
                kind(&chat_target, ResolvedChatKind::Supergroup),
                member(&chat_target, ChatMemberRight::CanPinMessages),
            ],
            vec![
                kind(&chat_target, ResolvedChatKind::Channel),
                administrator(&chat_target, ChatAdministratorRight::CanEditMessages),
            ],
        ],
    );
}

#[test]
fn reviewed_contract_stays_open_when_an_argument_signal_is_deferred() {
    let schema = Schema::parse(&SCHEMA.replace(
        "@message_id Message identifier @reason Diagnostic fixture\nunpinChatMessage",
        "@message_id Message identifier @reason Requires messageProperties.can_be_edited\nunpinChatMessage",
    ))
    .expect("fixture schema with an additional argument signal");

    let error = documented_runtime_requirements(find_method(&schema, "unpinChatMessage"))
        .expect_err("an exact description must not hide a deferred argument signal");
    assert_eq!(error.kind(), CapabilityGenerationErrorKind::SchemaDrift);
    assert!(
        error
            .to_string()
            .contains("at least one runtime signal still needs a typed disposition")
    );
}

#[test]
fn reviewed_contract_consumes_only_its_explicit_description_families() {
    let schema = Schema::parse(&SCHEMA.replace(
        "Requires administrator evidence in a synthetic supergroup fixture",
        "Requires administrator evidence and messageProperties.can_be_edited in a synthetic supergroup fixture",
    ))
    .expect("mixed description fixture");
    let consumed = [RuntimeSignalKey {
        source: RuntimeSignalSource::Description,
        family: RuntimeSignalFamily::RequiresAdministrator,
    }]
    .into_iter()
    .collect();

    let dispositions = runtime_signal_dispositions_with_consumed(
        find_method(&schema, "requireSyntheticSupergroupAdministrator"),
        &consumed,
    )
    .expect("schema-bound signal dispositions");
    assert_eq!(
        dispositions.into_iter().collect::<BTreeMap<_, _>>(),
        [
            (
                RuntimeSignalKey {
                    source: RuntimeSignalSource::Description,
                    family: RuntimeSignalFamily::RequiresAdministrator,
                },
                RuntimeSignalDisposition::ConsumedByRuntimeRequirements,
            ),
            (
                RuntimeSignalKey {
                    source: RuntimeSignalSource::Description,
                    family: RuntimeSignalFamily::MessagePropertiesFact,
                },
                RuntimeSignalDisposition::Deferred(DeferredSignalLane::UnclassifiedDescription,),
            ),
            (
                RuntimeSignalKey {
                    source: RuntimeSignalSource::Description,
                    family: RuntimeSignalFamily::CanFieldReference,
                },
                RuntimeSignalDisposition::Deferred(DeferredSignalLane::UnclassifiedDescription,),
            ),
        ]
        .into_iter()
        .collect()
    );
}

#[test]
fn runtime_signal_sources_must_be_unique_schema_arguments() {
    let assert_schema_drift = |schema: String, expected: &str| {
        let schema = Schema::parse(&schema).expect("signal-source fixture schema");
        let error =
            documented_runtime_signal_dispositions(find_method(&schema, "unpinChatMessage"))
                .expect_err("invalid signal source must fail closed");
        assert_eq!(error.kind(), CapabilityGenerationErrorKind::SchemaDrift);
        assert!(error.to_string().contains(expected), "{error}");
    };

    assert_schema_drift(
        SCHEMA.replace(
            "@reason Diagnostic fixture\nunpinChatMessage",
            "@reason messageProperties.can_be_edited @reason groupCall.can_be_managed\nunpinChatMessage",
        ),
        "runtime signal source tag is duplicated",
    );
    assert_schema_drift(
        SCHEMA.replace(
            "@reason Diagnostic fixture\nunpinChatMessage",
            "@unknown messageProperties.can_be_edited @reason Diagnostic fixture\nunpinChatMessage",
        ),
        "runtime signal belongs to neither @description nor a method argument",
    );
}

#[test]
fn pinned_chat_boost_vocabulary_is_terminal_non_gate_documentation() {
    let schema =
        Schema::parse(include_str!("../../../../vendor/tdlib/td_api.tl")).expect("pinned schema");

    for method in [
        "getChatBoostFeatures",
        "getChatBoostLevelFeatures",
        "getChatBoostLinkInfo",
    ] {
        assert_eq!(
            documented_runtime_requirements(find_method(&schema, method))
                .expect("reviewed lexical non-gate"),
            None,
            "{method}"
        );
    }
    assert_eq!(
        documented_runtime_signal_dispositions(find_method(&schema, "getChatBoostLinkInfo"))
            .expect("exact link-info disposition"),
        vec![(
            RuntimeSignalKey {
                source: RuntimeSignalSource::Description,
                family: RuntimeSignalFamily::ChatBoostReference,
            },
            RuntimeSignalDisposition::NotRuntimeGate(NonGateReason::ChatBoostVocabulary),
        )]
    );
}

#[test]
fn chat_boost_non_gate_requires_exact_reviewed_wording() {
    let pinned = include_str!("../../../../vendor/tdlib/td_api.tl");
    for (method, family, original, replacement) in [
        (
            "getChatBoostFeatures",
            RuntimeSignalFamily::BoostLevelPhrase,
            "Returns the list of features available for different chat boost levels. This is an offline method",
            "Requires boost level 1 before returning the list of features",
        ),
        (
            "getChatBoostLinkInfo",
            RuntimeSignalFamily::ChatBoostReference,
            "Returns information about a link to boost a chat. Can be called for any internal link of the type internalLinkTypeChatBoost",
            "Returns information about a link to boost a chat. Can be called only after chatBoost activation",
        ),
    ] {
        let schema = Schema::parse(&pinned.replace(original, replacement))
            .expect("same-family semantic drift fixture");
        let method = find_method(&schema, method);
        assert_eq!(
            documented_runtime_signal_dispositions(method).expect("mutated disposition"),
            vec![(
                RuntimeSignalKey {
                    source: RuntimeSignalSource::Description,
                    family,
                },
                RuntimeSignalDisposition::Deferred(DeferredSignalLane::UnclassifiedDescription),
            )],
            "mutation must preserve exactly one lexical family"
        );
        let error = documented_runtime_requirements(method)
            .expect_err("same lexical family must not inherit terminal non-gate status");
        assert_eq!(error.kind(), CapabilityGenerationErrorKind::SchemaDrift);
        assert!(
            error
                .to_string()
                .contains("at least one runtime signal still needs a typed disposition")
        );
    }
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
fn rejects_the_previous_capability_policy_format() {
    let fixture = Fixture::new(SCHEMA);
    let mut policy = fixture.capability_value();
    policy["format_version"] = json!(super::FORMAT_VERSION - 1);
    assert_policy_error(
        &fixture,
        policy,
        CapabilityGenerationErrorKind::InvalidPolicy,
    );
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
            "unknown chat kind",
            json!({"kind": "any_of", "clauses": [{"all_of": [{
                "kind": "chat_kind", "target_argument": "chat_id", "value": "group"
            }]}]}),
        ),
        (
            "contradictory chat kinds",
            json!({"kind": "any_of", "clauses": [{"all_of": [
                {"kind": "chat_kind", "target_argument": "chat_id", "value": "private"},
                {"kind": "chat_kind", "target_argument": "chat_id", "value": "channel"}
            ]}]}),
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

    let mut incompatible_kind_target = fixture.capability_value();
    method_row_mut(&mut incompatible_kind_target, "setSupergroupStickerSet")["runtime_requirements"] = json!({
        "kind": "any_of",
        "clauses": [{"all_of": [{
            "kind": "chat_kind",
            "target_argument": "supergroup_id",
            "value": "private"
        }]}]
    });
    assert_policy_error(
        &fixture,
        incompatible_kind_target,
        CapabilityGenerationErrorKind::InvalidPolicy,
    );

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
        "unpinChatMessage",
        "reportSupergroupSpam",
        "banGroupCallParticipants",
    ] {
        let error =
            validate_documented_runtime_requirements(find_method(&schema, method), &unrestricted)
                .expect_err(method);
        assert_eq!(error.kind(), CapabilityGenerationErrorKind::InvalidPolicy);
    }

    for method in [
        "createForumTopic",
        "editForumTopic",
        "deleteForumTopic",
        "setChatMemberStatus",
        "getSupergroupMembers",
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
            "format_version": super::FORMAT_VERSION,
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
                    {"all_of": [chat_kind("chat_id", "private")]},
                    {"all_of": [chat_kind("chat_id", "secret")]},
                    {"all_of": [
                        chat_kind("chat_id", "basic_group"),
                        chat_member_right("chat_id", "can_pin_messages")
                    ]},
                    {"all_of": [
                        chat_kind("chat_id", "supergroup"),
                        chat_member_right("chat_id", "can_pin_messages")
                    ]},
                    {"all_of": [
                        chat_kind("chat_id", "channel"),
                        chat_administrator_right("chat_id", "can_edit_messages")
                    ]}
                ]
            });
        }
        "toggleForumTopicIsClosed" => {
            row["runtime_requirements"] = json!({
                "kind": "any_of",
                "clauses": [
                    {"all_of": [
                        chat_kind("chat_id", "supergroup"),
                        chat_administrator_right("chat_id", "can_manage_topics")
                    ]},
                    {"all_of": [
                        chat_kind("chat_id", "supergroup"),
                        {
                            "kind": "topic_creator",
                            "target_argument": "chat_id",
                            "topic_argument": "forum_topic_id"
                        }
                    ]}
                ]
            });
        }
        "setSupergroupStickerSet" => {
            row["runtime_requirements"] = json!({
                "kind": "any_of",
                "clauses": [{"all_of": [
                    chat_kind("supergroup_id", "supergroup"),
                    chat_administrator_right("supergroup_id", "can_change_info")
                ]}]
            });
        }
        "requireSyntheticSupergroupAdministrator" => {
            row["runtime_requirements"] = json!({
                "kind": "any_of",
                "clauses": [{"all_of": [
                    chat_kind("supergroup_id", "supergroup"),
                    {
                        "kind": "chat_administrator",
                        "target_argument": "supergroup_id"
                    }
                ]}]
            });
        }
        "upgradeBasicGroupChatToSupergroupChat" => {
            row["ready_accounts"] = json!(["regular_user"]);
            row["runtime_requirements"] = json!({
                "kind": "any_of",
                "clauses": [{"all_of": [
                    chat_kind("chat_id", "basic_group"),
                    {
                        "kind": "chat_owner",
                        "target_argument": "chat_id"
                    }
                ]}]
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

fn chat_kind(target_argument: &str, value: &str) -> Value {
    json!({
        "kind": "chat_kind",
        "target_argument": target_argument,
        "value": value
    })
}

fn chat_administrator_right(target_argument: &str, right: &str) -> Value {
    json!({
        "kind": "chat_administrator_right",
        "target_argument": target_argument,
        "right": right
    })
}

fn chat_member_right(target_argument: &str, right: &str) -> Value {
    json!({
        "kind": "chat_member_right",
        "target_argument": target_argument,
        "right": right
    })
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
