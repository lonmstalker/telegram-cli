use std::collections::BTreeSet;

use crate::schema::{DefinitionKind, Parameter, Schema};

use super::{
    AccountKind, ApplicationRequirement, ArgumentRef, AuthorizationState, BusinessBotRight,
    BusinessConnectionRef, CapabilityDescriptor, CapabilityModelErrorKind, ChatAdministratorRight,
    ChatKindCondition, ChatMemberRight, ChatTargetRef, CurrentAccountEntitlement, DcEnvironment,
    ForumTopicRef, MAX_ATOMS_PER_METHOD, MAX_CLAUSES_PER_METHOD, MAX_PARAMETER_NOTICES_PER_METHOD,
    MAX_SYNCHRONOUS_VALUES_PER_METHOD, MessageCapability, MessageIdRef, MessageIdsRef,
    MessageSubjectRef, ParameterCapabilityNotice, ParameterGate, ParameterStringValue,
    RequirementAlternatives, ResolvedChatKind, RuntimeRequirement, SynchronousCapability,
};

#[test]
fn closed_capability_vocabularies_are_complete_and_canonical() {
    assert_eq!(AccountKind::ALL.len(), 2);
    assert_eq!(AuthorizationState::ALL.len(), 13);
    assert_eq!(ChatAdministratorRight::ALL.len(), 16);
    assert_eq!(ChatMemberRight::ALL.len(), 16);
    assert_eq!(BusinessBotRight::ALL.len(), 14);

    for account in AccountKind::ALL {
        assert_eq!(AccountKind::try_from(account.as_str()), Ok(account));
    }
    for state in AuthorizationState::ALL {
        assert_eq!(AuthorizationState::try_from(state.as_str()), Ok(state));
        assert!(state.as_str().starts_with("authorizationState"));
    }
    assert!(AccountKind::try_from("unknown").is_err());
    assert!(AuthorizationState::try_from("ready").is_err());
}

#[test]
fn rights_vocabularies_are_derived_from_the_pinned_schema() {
    let schema = Schema::parse(include_str!("../../../../vendor/tdlib/td_api.tl"))
        .expect("pinned schema parses");

    let mut administrator = schema_fields(&schema, "chatAdministratorRights");
    assert!(administrator.remove("is_anonymous"));
    assert_eq!(administrator, enum_values(ChatAdministratorRight::ALL));
    assert_eq!(
        schema_fields(&schema, "chatPermissions"),
        enum_values(ChatMemberRight::ALL)
    );
    assert_eq!(
        schema_fields(&schema, "businessBotRights"),
        enum_values(BusinessBotRight::ALL)
    );
}

#[test]
fn message_capabilities_and_subject_cardinality_are_closed_and_semantic() {
    assert_eq!(MessageCapability::ALL.len(), 36);
    assert_eq!(
        schema_fields(
            &Schema::parse(include_str!("../../../../vendor/tdlib/td_api.tl")).unwrap(),
            "messageProperties"
        )
        .into_iter()
        .filter(|field| field.starts_with("can_"))
        .collect::<BTreeSet<_>>(),
        enum_values(MessageCapability::ALL)
    );
    for capability in MessageCapability::ALL {
        assert_eq!(
            MessageCapability::try_from(capability.as_str()),
            Ok(capability)
        );
    }
    assert!(MessageCapability::try_from("need_show_statistics").is_err());

    let one = MessageSubjectRef::One {
        chat: ChatTargetRef::try_from("chat_id").expect("chat target"),
        message: MessageIdRef::try_from("message_id").expect("message id"),
    };
    let each = MessageSubjectRef::Each {
        chat: ChatTargetRef::try_from("supergroup_id").expect("supergroup target"),
        messages: MessageIdsRef::try_from("message_ids").expect("message ids"),
    };
    assert_eq!(one.chat().argument().as_str(), "chat_id");
    assert_eq!(one.message_argument().as_str(), "message_id");
    assert_eq!(each.chat().argument().as_str(), "supergroup_id");
    assert_eq!(each.message_argument().as_str(), "message_ids");
    assert!(MessageIdRef::try_from("message_ids").is_err());
    assert!(MessageIdsRef::try_from("message_id").is_err());

    let requirement = RuntimeRequirement::MessageCapability {
        subject: each,
        capability: MessageCapability::CanReportSupergroupSpam,
    };
    assert_eq!(
        requirement
            .argument_refs()
            .into_iter()
            .map(ArgumentRef::as_str)
            .collect::<Vec<_>>(),
        ["supergroup_id", "message_ids"]
    );
}

#[test]
fn descriptor_keeps_orthogonal_capability_axes_without_inventing_account_state() {
    let requirement = RuntimeRequirement::BusinessConnectionRight {
        connection: BusinessConnectionRef::try_from("business_connection_id")
            .expect("business connection"),
        right: BusinessBotRight::CanReply,
    };
    let descriptor = CapabilityDescriptor::try_new(
        SynchronousCapability::Never,
        vec![AccountKind::Bot],
        vec![AuthorizationState::Ready],
        Vec::new(),
        ApplicationRequirement::Any,
        vec![DcEnvironment::Test, DcEnvironment::Production],
        RequirementAlternatives::try_new(vec![vec![requirement.clone()]]).expect("alternatives"),
        Vec::new(),
    )
    .expect("valid bot capability");

    assert_eq!(descriptor.synchronous(), &SynchronousCapability::Never);
    assert_eq!(descriptor.ready_accounts(), &[AccountKind::Bot]);
    assert_eq!(
        descriptor.authorization_states(),
        &[AuthorizationState::Ready]
    );
    assert_eq!(
        descriptor.dc_environments(),
        &[DcEnvironment::Production, DcEnvironment::Test]
    );
    assert_eq!(
        descriptor.runtime_requirements().clauses(),
        &[vec![requirement]]
    );
    assert!(descriptor.parameter_notices().is_empty());

    let pre_auth = CapabilityDescriptor::try_new(
        SynchronousCapability::Never,
        Vec::new(),
        vec![AuthorizationState::WaitPhoneNumber],
        Vec::new(),
        ApplicationRequirement::Official,
        vec![DcEnvironment::Production, DcEnvironment::Test],
        RequirementAlternatives::always(),
        Vec::new(),
    )
    .expect("pre-auth capability has no invented account kind");
    assert!(pre_auth.ready_accounts().is_empty());

    let synchronous = CapabilityDescriptor::try_new(
        SynchronousCapability::Always,
        Vec::new(),
        vec![AuthorizationState::WaitTdlibParameters],
        Vec::new(),
        ApplicationRequirement::Any,
        vec![DcEnvironment::Production, DcEnvironment::Test],
        RequirementAlternatives::always(),
        Vec::new(),
    )
    .expect("synchronous is additive to an exact client auth capability");
    assert_eq!(synchronous.synchronous(), &SynchronousCapability::Always);

    let conditional = SynchronousCapability::for_string_values(
        "name".try_into().expect("parameter"),
        vec![
            ParameterStringValue::try_from("version").expect("value"),
            ParameterStringValue::try_from("commit_hash").expect("value"),
        ],
    )
    .expect("getOption-style condition");
    assert!(matches!(
        conditional,
        SynchronousCapability::StringParameterValues(condition)
            if condition.values()[0].as_str() == "commit_hash"
    ));
}

#[test]
fn descriptor_rejects_inconsistent_or_ambiguous_capability_claims() {
    let base = || {
        CapabilityDescriptor::try_new(
            SynchronousCapability::Never,
            vec![AccountKind::RegularUser],
            vec![AuthorizationState::Ready],
            Vec::new(),
            ApplicationRequirement::Any,
            vec![DcEnvironment::Production, DcEnvironment::Test],
            RequirementAlternatives::always(),
            Vec::new(),
        )
    };
    assert!(base().is_ok());

    assert_eq!(
        CapabilityDescriptor::try_new(
            SynchronousCapability::Never,
            Vec::new(),
            vec![AuthorizationState::Ready],
            Vec::new(),
            ApplicationRequirement::Any,
            vec![DcEnvironment::Production],
            RequirementAlternatives::always(),
            Vec::new(),
        )
        .expect_err("Ready needs an account set")
        .kind(),
        CapabilityModelErrorKind::InconsistentReadyAccounts
    );
    assert_eq!(
        CapabilityDescriptor::try_new(
            SynchronousCapability::Never,
            vec![AccountKind::Bot],
            vec![AuthorizationState::Ready],
            vec![CurrentAccountEntitlement::Premium],
            ApplicationRequirement::Any,
            vec![DcEnvironment::Production],
            RequirementAlternatives::always(),
            Vec::new(),
        )
        .expect_err("current-account entitlement is not a bot capability")
        .kind(),
        CapabilityModelErrorKind::IncompatibleEntitlement
    );
    assert_eq!(
        CapabilityDescriptor::try_new(
            SynchronousCapability::Never,
            vec![AccountKind::RegularUser, AccountKind::RegularUser],
            vec![AuthorizationState::Ready],
            Vec::new(),
            ApplicationRequirement::Any,
            vec![DcEnvironment::Production],
            RequirementAlternatives::always(),
            Vec::new(),
        )
        .expect_err("duplicate account kind")
        .kind(),
        CapabilityModelErrorKind::DuplicateValue
    );

    let business_requirement = RequirementAlternatives::try_new(vec![vec![
        RuntimeRequirement::BusinessConnectionEnabled {
            connection: BusinessConnectionRef::try_from("business_connection_id")
                .expect("business connection"),
        },
    ]])
    .expect("bounded runtime requirement");
    assert_eq!(
        CapabilityDescriptor::try_new(
            SynchronousCapability::Never,
            vec![AccountKind::RegularUser],
            vec![AuthorizationState::Ready],
            Vec::new(),
            ApplicationRequirement::Any,
            vec![DcEnvironment::Production],
            business_requirement,
            Vec::new(),
        )
        .expect_err("a regular user can't satisfy a business-connection requirement")
        .kind(),
        CapabilityModelErrorKind::IncompatibleRuntimeRequirement
    );

    let owner_requirement =
        RequirementAlternatives::try_new(vec![vec![RuntimeRequirement::ChatOwner {
            target: ChatTargetRef::try_from("chat_id").expect("chat target"),
        }]])
        .expect("bounded runtime requirement");
    assert_eq!(
        CapabilityDescriptor::try_new(
            SynchronousCapability::Never,
            vec![AccountKind::Bot],
            vec![AuthorizationState::Ready],
            Vec::new(),
            ApplicationRequirement::Any,
            vec![DcEnvironment::Production],
            owner_requirement,
            Vec::new(),
        )
        .expect_err("a bot can't own a chat")
        .kind(),
        CapabilityModelErrorKind::IncompatibleRuntimeRequirement
    );
}

#[test]
fn runtime_alternatives_and_parameter_notices_are_bounded_and_unambiguous() {
    assert_eq!(
        RequirementAlternatives::try_new(Vec::new())
            .expect_err("empty alternatives must use the explicit always constructor")
            .kind(),
        CapabilityModelErrorKind::EmptySet
    );
    assert_eq!(
        RequirementAlternatives::try_new(vec![Vec::new()])
            .expect_err("empty clause")
            .kind(),
        CapabilityModelErrorKind::EmptyClause
    );
    let requirement = RuntimeRequirement::ChatAdministratorRight {
        target: ChatTargetRef::try_from("chat_id").expect("chat target"),
        right: ChatAdministratorRight::CanManageVideoChats,
    };
    assert_eq!(
        RequirementAlternatives::try_new(vec![vec![requirement.clone(), requirement]])
            .expect_err("duplicate atom")
            .kind(),
        CapabilityModelErrorKind::DuplicateValue
    );
    let broad = RuntimeRequirement::ChatOwner {
        target: ChatTargetRef::try_from("supergroup_id").expect("supergroup target"),
    };
    let narrow = RuntimeRequirement::TopicCreator {
        target: ChatTargetRef::try_from("supergroup_id").expect("supergroup target"),
        topic: ForumTopicRef::try_from("forum_topic_id").expect("forum topic"),
    };
    assert_eq!(
        RequirementAlternatives::try_new(vec![vec![broad.clone()], vec![broad, narrow],])
            .expect_err("the stricter clause is absorbed")
            .kind(),
        CapabilityModelErrorKind::RedundantClause
    );

    let administrator = RuntimeRequirement::ChatAdministrator {
        target: ChatTargetRef::try_from("chat_id").expect("chat target"),
    };
    assert_eq!(
        administrator.argument_refs()[0].as_str(),
        "chat_id",
        "generic administrator evidence keeps its semantic target"
    );
    assert!(ChatTargetRef::try_from("message_id").is_err());
    assert!(ChatTargetRef::try_from("other_supergroup_id").is_err());
    assert!(ForumTopicRef::try_from("other_topic_id").is_err());
    assert!(BusinessConnectionRef::try_from("reason").is_err());
    assert!(
        BusinessConnectionRef::try_from("connection_id").is_err(),
        "connection_id is ambiguous across pinned TDLib methods without method/type context"
    );

    assert!(super::ArgumentRef::try_from("chat_id").is_ok());
    assert!(super::ArgumentRef::try_from("chatId").is_err());
    assert!(super::ArgumentRef::try_from("").is_err());
    assert!(super::ArgumentRef::try_from(&"x".repeat(65)).is_err());
    assert_eq!(
        ParameterCapabilityNotice::try_new(
            "value".try_into().expect("argument"),
            ParameterGate::Application(ApplicationRequirement::Any),
        )
        .expect_err("an Any notice says nothing")
        .kind(),
        CapabilityModelErrorKind::VacuousParameterGate
    );

    let premium_notice = ParameterCapabilityNotice::try_new(
        "value".try_into().expect("argument"),
        ParameterGate::CurrentAccountEntitlement(CurrentAccountEntitlement::Premium),
    )
    .expect("notice");
    assert_eq!(
        CapabilityDescriptor::try_new(
            SynchronousCapability::Never,
            vec![AccountKind::Bot],
            vec![AuthorizationState::Ready],
            Vec::new(),
            ApplicationRequirement::Any,
            vec![DcEnvironment::Production, DcEnvironment::Test],
            RequirementAlternatives::always(),
            vec![premium_notice],
        )
        .expect_err("a bot account can't satisfy current-account Premium")
        .kind(),
        CapabilityModelErrorKind::IncompatibleParameterGate
    );

    let test_notice = ParameterCapabilityNotice::try_new(
        "value".try_into().expect("argument"),
        ParameterGate::DcEnvironment(DcEnvironment::Test),
    )
    .expect("notice");
    assert_eq!(
        CapabilityDescriptor::try_new(
            SynchronousCapability::Never,
            vec![AccountKind::RegularUser],
            vec![AuthorizationState::Ready],
            Vec::new(),
            ApplicationRequirement::Any,
            vec![DcEnvironment::Test],
            RequirementAlternatives::always(),
            vec![test_notice],
        )
        .expect_err("the whole method is already Test-only")
        .kind(),
        CapabilityModelErrorKind::RedundantParameterGate
    );
}

#[test]
fn chat_kind_conditions_are_closed_semantic_and_non_contradictory() {
    assert_eq!(ResolvedChatKind::ALL.len(), 5);
    for kind in ResolvedChatKind::ALL {
        assert_eq!(ResolvedChatKind::try_from(kind.as_str()), Ok(kind));
    }
    assert!(ResolvedChatKind::try_from("group").is_err());

    let chat = ChatTargetRef::try_from("chat_id").expect("chat target");
    let private = ChatKindCondition::try_new(chat.clone(), ResolvedChatKind::Private)
        .expect("chat_id can resolve to a private chat");
    assert_eq!(private.target(), &chat);
    assert_eq!(private.kind(), ResolvedChatKind::Private);

    let supergroup = ChatTargetRef::try_from("supergroup_id").expect("supergroup target");
    assert!(ChatKindCondition::try_new(supergroup.clone(), ResolvedChatKind::Supergroup).is_ok());
    assert!(ChatKindCondition::try_new(supergroup.clone(), ResolvedChatKind::Channel).is_ok());
    assert_eq!(
        ChatKindCondition::try_new(supergroup, ResolvedChatKind::Private)
            .expect_err("supergroup_id can't address a private chat")
            .kind(),
        CapabilityModelErrorKind::IncompatibleChatKindTarget
    );

    let private_atom = RuntimeRequirement::ChatKind(private);
    let channel_atom = RuntimeRequirement::ChatKind(
        ChatKindCondition::try_new(chat, ResolvedChatKind::Channel).expect("chat condition"),
    );
    assert_eq!(
        RequirementAlternatives::try_new(vec![vec![private_atom, channel_atom]])
            .expect_err("one target can't resolve to two chat kinds in one AND clause")
            .kind(),
        CapabilityModelErrorKind::ContradictoryClause
    );
}

#[test]
fn public_domain_constructors_enforce_collection_caps_before_canonicalization() {
    let requirements = unique_runtime_requirements();
    let max_clauses = requirements
        .iter()
        .take(MAX_CLAUSES_PER_METHOD)
        .cloned()
        .map(|requirement| vec![requirement])
        .collect::<Vec<_>>();
    assert!(RequirementAlternatives::try_new(max_clauses).is_ok());

    let too_many_clauses = requirements
        .iter()
        .take(MAX_CLAUSES_PER_METHOD + 1)
        .cloned()
        .map(|requirement| vec![requirement])
        .collect::<Vec<_>>();
    assert_eq!(
        RequirementAlternatives::try_new(too_many_clauses)
            .expect_err("clause cap")
            .kind(),
        CapabilityModelErrorKind::ResourceLimit
    );

    assert!(
        RequirementAlternatives::try_new(vec![requirements[..MAX_ATOMS_PER_METHOD].to_vec()])
            .is_ok()
    );
    assert_eq!(
        RequirementAlternatives::try_new(vec![requirements[..=MAX_ATOMS_PER_METHOD].to_vec()])
            .expect_err("atom cap")
            .kind(),
        CapabilityModelErrorKind::ResourceLimit
    );

    let string_values = |count| {
        (0..count)
            .map(|index| {
                ParameterStringValue::try_from(format!("value_{index}").as_str())
                    .expect("bounded unique value")
            })
            .collect::<Vec<_>>()
    };
    assert!(
        SynchronousCapability::for_string_values(
            ArgumentRef::try_from("name").expect("argument"),
            string_values(MAX_SYNCHRONOUS_VALUES_PER_METHOD),
        )
        .is_ok()
    );
    assert_eq!(
        SynchronousCapability::for_string_values(
            ArgumentRef::try_from("name").expect("argument"),
            string_values(MAX_SYNCHRONOUS_VALUES_PER_METHOD + 1),
        )
        .expect_err("synchronous value cap")
        .kind(),
        CapabilityModelErrorKind::ResourceLimit
    );

    let notices = |count| {
        (0..count)
            .map(|index| {
                let parameter = format!("value_{index}");
                ParameterCapabilityNotice::try_new(
                    ArgumentRef::try_from(&parameter).expect("bounded unique argument"),
                    ParameterGate::Account(AccountKind::Bot),
                )
                .expect("reachable notice")
            })
            .collect::<Vec<_>>()
    };
    let descriptor = |parameter_notices| {
        CapabilityDescriptor::try_new(
            SynchronousCapability::Never,
            vec![AccountKind::RegularUser, AccountKind::Bot],
            vec![AuthorizationState::Ready],
            Vec::new(),
            ApplicationRequirement::Any,
            vec![DcEnvironment::Production, DcEnvironment::Test],
            RequirementAlternatives::always(),
            parameter_notices,
        )
    };
    assert!(descriptor(notices(MAX_PARAMETER_NOTICES_PER_METHOD)).is_ok());
    assert_eq!(
        descriptor(notices(MAX_PARAMETER_NOTICES_PER_METHOD + 1))
            .expect_err("parameter notice cap")
            .kind(),
        CapabilityModelErrorKind::ResourceLimit
    );
}

fn unique_runtime_requirements() -> Vec<RuntimeRequirement> {
    let target = ChatTargetRef::try_from("chat_id").expect("chat target");
    let mut requirements = ChatAdministratorRight::ALL
        .into_iter()
        .map(|right| RuntimeRequirement::ChatAdministratorRight {
            target: target.clone(),
            right,
        })
        .collect::<Vec<_>>();
    requirements.extend(ChatMemberRight::ALL.into_iter().map(|right| {
        RuntimeRequirement::ChatMemberRight {
            target: target.clone(),
            right,
        }
    }));
    requirements.push(RuntimeRequirement::ChatAdministrator { target });
    requirements
}

fn schema_fields<'a>(schema: &'a Schema, name: &str) -> BTreeSet<&'a str> {
    let definition = schema
        .definitions()
        .iter()
        .find(|definition| {
            definition.kind() == DefinitionKind::Constructor && definition.name() == name
        })
        .unwrap_or_else(|| panic!("missing constructor {name}"));
    definition
        .parameters()
        .iter()
        .filter_map(|parameter| match parameter {
            Parameter::Field { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect()
}

fn enum_values<T: Copy + CapabilityValue, const N: usize>(
    values: [T; N],
) -> BTreeSet<&'static str> {
    values.into_iter().map(CapabilityValue::value).collect()
}

trait CapabilityValue {
    fn value(self) -> &'static str;
}

macro_rules! impl_capability_value {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl CapabilityValue for $ty {
                fn value(self) -> &'static str {
                    self.as_str()
                }
            }
        )+
    };
}

impl_capability_value!(
    ChatAdministratorRight,
    ChatMemberRight,
    BusinessBotRight,
    MessageCapability
);
