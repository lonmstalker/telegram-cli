//! Pure, bounded generation of schema-bound TDLib method capabilities.
//!
//! This module classifies static requirements. It never claims that a runtime
//! account currently satisfies them and it does not grant policy permission.

mod chat_boosts;
mod chat_event_logs;
mod chat_invite_links;
mod chat_settings;
mod message_moderation;
mod supergroup_usernames;
mod video_chats;

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Write};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use telegram_core::method_capability::{
    AccountKind, ApplicationRequirement, ArgumentRef, AuthorizationState, BusinessBotRight,
    BusinessConnectionRef, CapabilityDescriptor, ChatAdministratorRight, ChatKindCondition,
    ChatMemberRight, ChatTargetKind, ChatTargetRef, CurrentAccountEntitlement, DcEnvironment,
    ForumTopicRef, GroupCallIdRef, GroupCallKindCondition, GroupCallMessageCapability,
    GroupCallMessageIdsRef, GroupCallMessageSubjectRef, GroupCallProperty, MAX_ATOMS_PER_METHOD,
    MAX_CLAUSES_PER_METHOD, MAX_PARAMETER_NOTICES_PER_METHOD, MAX_SYNCHRONOUS_VALUES_PER_METHOD,
    MessageCapability, MessageIdRef, MessageIdsRef, MessageSubjectRef, ParameterCapabilityNotice,
    ParameterGate, ParameterStringValue, RequirementAlternatives, ResolvedChatKind,
    ResolvedGroupCallKind, RuntimeBooleanOption, RuntimeRequirement, SupergroupFlag,
    SupergroupFlagCondition, SupergroupFullInfoProperty, SynchronousCapability,
};
use telegram_core::schema::{Definition, DefinitionKind, Parameter, Schema};

const FORMAT_VERSION: u32 = 8;
const MAX_SCHEMA_BYTES: usize = 2 * 1024 * 1024;
const MAX_CAPABILITY_POLICY_BYTES: usize = 4 * 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_METHODS: usize = 2_048;
const MAX_RATIONALE_BYTES: usize = 1_024;

pub fn generate(
    schema_bytes: &[u8],
    capability_policy_bytes: &[u8],
) -> Result<Vec<u8>, CapabilityGenerationError> {
    enforce_cap("TDLib schema", schema_bytes.len(), MAX_SCHEMA_BYTES)?;
    enforce_cap(
        "capability policy",
        capability_policy_bytes.len(),
        MAX_CAPABILITY_POLICY_BYTES,
    )?;

    let schema_source = std::str::from_utf8(schema_bytes).map_err(|error| {
        CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::InvalidSchema,
            format!("TDLib schema is not UTF-8: {error}"),
        )
    })?;
    let schema = Schema::parse(schema_source).map_err(|error| {
        CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::InvalidSchema,
            error.to_string(),
        )
    })?;
    validate_authorization_states(&schema)?;
    validate_right_vocabularies(&schema)?;
    validate_chat_type_vocabulary(&schema)?;
    validate_message_properties_vocabulary(&schema)?;
    validate_group_call_vocabulary(&schema)?;
    validate_supergroup_vocabulary(&schema)?;
    validate_supergroup_full_info_vocabulary(&schema)?;
    validate_runtime_boolean_option_vocabulary(&schema)?;

    let policy: CapabilityPolicyDto =
        serde_json::from_slice(capability_policy_bytes).map_err(|error| {
            CapabilityGenerationError::invalid_policy(format!("invalid capability policy: {error}"))
        })?;
    build_output(schema_bytes, schema, policy)
}

fn build_output(
    schema_bytes: &[u8],
    schema: Schema,
    policy: CapabilityPolicyDto,
) -> Result<Vec<u8>, CapabilityGenerationError> {
    if policy.format_version != FORMAT_VERSION {
        return Err(CapabilityGenerationError::invalid_policy(format!(
            "unsupported capability policy format_version {}",
            policy.format_version
        )));
    }
    validate_hash("schema_sha256", &policy.schema_sha256)?;
    let actual_schema_hash = sha256_hex(schema_bytes);
    if policy.schema_sha256 != actual_schema_hash {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            "capability policy is bound to a different schema hash",
        ));
    }
    if policy.methods.len() > MAX_METHODS {
        return Err(CapabilityGenerationError::resource_limit(format!(
            "capability policy has {} methods, exceeding the {MAX_METHODS}-method cap",
            policy.methods.len()
        )));
    }

    let methods = schema
        .methods()
        .iter()
        .map(|method| (method.name(), method))
        .collect::<BTreeMap<_, _>>();
    if methods.len() > MAX_METHODS {
        return Err(CapabilityGenerationError::resource_limit(format!(
            "schema has {} methods, exceeding the {MAX_METHODS}-method cap",
            methods.len()
        )));
    }
    let mut policy_rows = BTreeMap::new();
    for row in policy.methods {
        if policy_rows.insert(row.method.clone(), row).is_some() {
            return Err(CapabilityGenerationError::invalid_policy(
                "capability policy contains a duplicate method row",
            ));
        }
    }
    if policy_rows.len() != methods.len()
        || policy_rows
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            != methods.keys().copied().collect::<BTreeSet<_>>()
    {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::Coverage,
            "capability policy method set must exactly equal the schema method set",
        ));
    }

    let mut rows = Vec::with_capacity(methods.len());
    for (method_name, definition) in methods {
        let policy_row = policy_rows
            .remove(method_name)
            .expect("exact method-set equality was checked");
        rows.push(build_method_row(definition, policy_row)?);
    }

    let canonical_policy = CanonicalPolicy {
        format_version: FORMAT_VERSION,
        schema_sha256: &policy.schema_sha256,
        methods: &rows,
    };
    let semantic_policy = compact_json(&canonical_policy, "capability policy")?;
    let mapping_bytes = compact_json(&rows, "capability mapping")?;
    let output = GeneratedManifest {
        format_version: FORMAT_VERSION,
        generated_by: "tdlib-registry-gen/capability",
        engine_source_sha256: engine_source_sha256(),
        schema: SchemaEvidence {
            sha256: policy.schema_sha256,
            methods: rows.len(),
            authorization_states: AuthorizationState::ALL.len(),
        },
        policy: PolicyEvidence {
            semantic_sha256: sha256_hex(&semantic_policy),
        },
        counts: Counts {
            schema_methods: rows.len(),
            capability_methods: rows.len(),
        },
        mapping_sha256: sha256_hex(&mapping_bytes),
        methods: rows,
    };
    serialize_pretty_with_limit(&output, MAX_OUTPUT_BYTES)
}

fn build_method_row(
    method: &Definition,
    policy: MethodPolicyDto,
) -> Result<CanonicalMethodRow, CapabilityGenerationError> {
    validate_hash("method signature_sha256", &policy.signature_sha256)?;
    validate_hash("method documentation_sha256", &policy.documentation_sha256)?;
    validate_rationale(&policy.rationale)?;
    let signature_sha256 = sha256_hex(method.canonical_signature().as_bytes());
    if policy.signature_sha256 != signature_sha256 {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            format!("stale signature evidence for {:?}", method.name()),
        ));
    }
    let documentation_sha256 = documentation_sha256(method);
    if policy.documentation_sha256 != documentation_sha256 {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            format!("stale documentation evidence for {:?}", method.name()),
        ));
    }
    let synchronous = parse_synchronous(policy.synchronous, method)?;
    let ready_accounts = parse_values(
        "ready_accounts",
        policy.ready_accounts,
        AccountKind::ALL.len(),
        |value| AccountKind::try_from(value),
    )?;
    let authorization_states = parse_values(
        "authorization_states",
        policy.authorization_states,
        AuthorizationState::ALL.len(),
        |value| AuthorizationState::try_from(value),
    )?;
    let current_account_entitlements = parse_values(
        "current_account_entitlements",
        policy.current_account_entitlements,
        CurrentAccountEntitlement::ALL.len(),
        |value| CurrentAccountEntitlement::try_from(value),
    )?;
    let application = ApplicationRequirement::try_from(policy.application.as_str())
        .map_err(CapabilityGenerationError::from_model_value)?;
    let dc_environments = parse_values(
        "dc_environments",
        policy.dc_environments,
        DcEnvironment::ALL.len(),
        |value| DcEnvironment::try_from(value),
    )?;
    let runtime_requirements = parse_runtime_requirements(policy.runtime_requirements, method)?;
    let parameter_notices = parse_parameter_notices(policy.parameter_notices, method)?;
    let descriptor = CapabilityDescriptor::try_new(
        synchronous,
        ready_accounts,
        authorization_states,
        current_account_entitlements,
        application,
        dc_environments,
        runtime_requirements,
        parameter_notices,
    )
    .map_err(|error| {
        CapabilityGenerationError::invalid_policy(format!(
            "invalid capability for {:?}: {error}",
            method.name()
        ))
    })?;
    validate_documented_authorization_states(method, &descriptor)?;
    validate_documented_method_constraints(method, &descriptor)?;
    validate_documented_runtime_requirements(method, &descriptor)?;
    validate_documented_parameter_notices(method, &descriptor)?;

    Ok(CanonicalMethodRow::from_descriptor(
        method.name().to_owned(),
        signature_sha256,
        documentation_sha256,
        descriptor,
        policy.rationale,
    ))
}

fn parse_synchronous(
    dto: SynchronousDto,
    method: &Definition,
) -> Result<SynchronousCapability, CapabilityGenerationError> {
    let capability = match dto {
        SynchronousDto::Never => SynchronousCapability::Never,
        SynchronousDto::Always => SynchronousCapability::Always,
        SynchronousDto::StringParameterValues { parameter, values } => {
            if values.is_empty() || values.len() > MAX_SYNCHRONOUS_VALUES_PER_METHOD {
                return Err(CapabilityGenerationError::resource_limit(format!(
                    "conditional synchronous capability for {:?} needs 1..={MAX_SYNCHRONOUS_VALUES_PER_METHOD} values",
                    method.name()
                )));
            }
            let parameter = ArgumentRef::try_from(parameter.as_str())
                .map_err(|error| CapabilityGenerationError::invalid_policy(error.to_string()))?;
            require_argument_type(method, &parameter, "string")?;
            let string_parameters = method
                .parameters()
                .iter()
                .filter_map(|candidate| match candidate {
                    Parameter::Field { name, ty }
                        if ty.name() == "string" && ty.arguments().is_empty() =>
                    {
                        Some(name.as_str())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            if string_parameters.as_slice() != [parameter.as_str()] {
                return Err(CapabilityGenerationError::invalid_policy(format!(
                    "conditional synchronous capability for {:?} needs one unambiguous string parameter",
                    method.name()
                )));
            }
            let values = values
                .iter()
                .map(|value| {
                    ParameterStringValue::try_from(value.as_str()).map_err(|error| {
                        CapabilityGenerationError::invalid_policy(error.to_string())
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            SynchronousCapability::for_string_values(parameter, values)
                .map_err(|error| CapabilityGenerationError::invalid_policy(error.to_string()))?
        }
    };
    validate_synchronous_documentation(method, &capability)?;
    Ok(capability)
}

fn parse_runtime_requirements(
    dto: RuntimeRequirementsDto,
    method: &Definition,
) -> Result<RequirementAlternatives, CapabilityGenerationError> {
    match dto {
        RuntimeRequirementsDto::Always => Ok(RequirementAlternatives::always()),
        RuntimeRequirementsDto::AnyOf { clauses } => {
            if clauses.is_empty() {
                return Err(CapabilityGenerationError::invalid_policy(format!(
                    "runtime requirements for {:?} need at least one clause",
                    method.name()
                )));
            }
            if clauses.len() > MAX_CLAUSES_PER_METHOD {
                return Err(CapabilityGenerationError::resource_limit(format!(
                    "runtime requirements for {:?} exceed the {MAX_CLAUSES_PER_METHOD}-clause cap",
                    method.name()
                )));
            }
            let atom_count = clauses
                .iter()
                .map(|clause| clause.all_of.len())
                .sum::<usize>();
            if atom_count > MAX_ATOMS_PER_METHOD {
                return Err(CapabilityGenerationError::resource_limit(format!(
                    "runtime requirements for {:?} exceed the {MAX_ATOMS_PER_METHOD}-atom cap",
                    method.name()
                )));
            }
            let clauses = clauses
                .into_iter()
                .map(|clause| {
                    clause
                        .all_of
                        .into_iter()
                        .map(|requirement| parse_runtime_requirement(requirement, method))
                        .collect::<Result<Vec<_>, _>>()
                })
                .collect::<Result<Vec<_>, _>>()?;
            RequirementAlternatives::try_new(clauses).map_err(|error| {
                CapabilityGenerationError::invalid_policy(format!(
                    "invalid runtime requirements for {:?}: {error}",
                    method.name()
                ))
            })
        }
    }
}

fn parse_runtime_requirement(
    dto: RuntimeRequirementDto,
    method: &Definition,
) -> Result<RuntimeRequirement, CapabilityGenerationError> {
    match dto {
        RuntimeRequirementDto::ChatKind {
            target_argument,
            value,
        } => {
            let target = parse_chat_target(method, target_argument)?;
            let kind = ResolvedChatKind::try_from(value.as_str())
                .map_err(CapabilityGenerationError::from_model_value)?;
            let condition = ChatKindCondition::try_new(target, kind)
                .map_err(CapabilityGenerationError::from_model_value)?;
            Ok(RuntimeRequirement::ChatKind(condition))
        }
        RuntimeRequirementDto::SupergroupFlag {
            target_argument,
            flag,
            value,
        } => {
            let target = parse_chat_target(method, target_argument)?;
            let flag = SupergroupFlag::try_from(flag.as_str())
                .map_err(CapabilityGenerationError::from_model_value)?;
            Ok(RuntimeRequirement::SupergroupFlag(
                SupergroupFlagCondition::new(target, flag, value),
            ))
        }
        RuntimeRequirementDto::ChatAdministrator { target_argument } => {
            Ok(RuntimeRequirement::ChatAdministrator {
                target: parse_chat_target(method, target_argument)?,
            })
        }
        RuntimeRequirementDto::ChatAdministratorRight {
            target_argument,
            right,
        } => {
            let target = parse_chat_target(method, target_argument)?;
            let right = ChatAdministratorRight::try_from(right.as_str())
                .map_err(CapabilityGenerationError::from_model_value)?;
            Ok(RuntimeRequirement::ChatAdministratorRight { target, right })
        }
        RuntimeRequirementDto::ChatMemberRight {
            target_argument,
            right,
        } => {
            let target = parse_chat_target(method, target_argument)?;
            let right = ChatMemberRight::try_from(right.as_str())
                .map_err(CapabilityGenerationError::from_model_value)?;
            Ok(RuntimeRequirement::ChatMemberRight { target, right })
        }
        RuntimeRequirementDto::ChatOwner { target_argument } => Ok(RuntimeRequirement::ChatOwner {
            target: parse_chat_target(method, target_argument)?,
        }),
        RuntimeRequirementDto::TopicCreator {
            target_argument,
            topic_argument,
        } => Ok(RuntimeRequirement::TopicCreator {
            target: parse_chat_target(method, target_argument)?,
            topic: ForumTopicRef::try_from(parse_role_argument(
                method,
                topic_argument,
                &["forum_topic_id"],
                "int32",
            )?)
            .map_err(|error| CapabilityGenerationError::invalid_policy(error.to_string()))?,
        }),
        RuntimeRequirementDto::BusinessConnectionEnabled {
            connection_argument,
        } => Ok(RuntimeRequirement::BusinessConnectionEnabled {
            connection: BusinessConnectionRef::try_from(parse_role_argument(
                method,
                connection_argument,
                &["business_connection_id"],
                "string",
            )?)
            .map_err(|error| CapabilityGenerationError::invalid_policy(error.to_string()))?,
        }),
        RuntimeRequirementDto::BusinessConnectionRight {
            connection_argument,
            right,
        } => {
            let right = BusinessBotRight::try_from(right.as_str())
                .map_err(CapabilityGenerationError::from_model_value)?;
            Ok(RuntimeRequirement::BusinessConnectionRight {
                connection: BusinessConnectionRef::try_from(parse_role_argument(
                    method,
                    connection_argument,
                    &["business_connection_id"],
                    "string",
                )?)
                .map_err(|error| CapabilityGenerationError::invalid_policy(error.to_string()))?,
                right,
            })
        }
        RuntimeRequirementDto::MessageCapability {
            subject,
            capability,
        } => Ok(RuntimeRequirement::MessageCapability {
            subject: parse_message_subject(method, subject)?,
            capability: MessageCapability::try_from(capability.as_str())
                .map_err(CapabilityGenerationError::from_model_value)?,
        }),
        RuntimeRequirementDto::GroupCallKind {
            group_call_argument,
            value,
        } => Ok(RuntimeRequirement::GroupCallKind(
            GroupCallKindCondition::new(
                parse_group_call_id(method, group_call_argument)?,
                ResolvedGroupCallKind::try_from(value.as_str())
                    .map_err(CapabilityGenerationError::from_model_value)?,
            ),
        )),
        RuntimeRequirementDto::GroupCallProperty {
            group_call_argument,
            property,
        } => Ok(RuntimeRequirement::GroupCallProperty {
            group_call: parse_group_call_id(method, group_call_argument)?,
            property: GroupCallProperty::try_from(property.as_str())
                .map_err(CapabilityGenerationError::from_model_value)?,
        }),
        RuntimeRequirementDto::GroupCallMessageCapability {
            subject,
            capability,
        } => Ok(RuntimeRequirement::GroupCallMessageCapability {
            subject: parse_group_call_message_subject(method, subject)?,
            capability: GroupCallMessageCapability::try_from(capability.as_str())
                .map_err(CapabilityGenerationError::from_model_value)?,
        }),
        RuntimeRequirementDto::SupergroupFullInfoProperty {
            target_argument,
            property,
        } => Ok(RuntimeRequirement::SupergroupFullInfoProperty {
            target: parse_chat_target(method, target_argument)?,
            property: SupergroupFullInfoProperty::try_from(property.as_str())
                .map_err(CapabilityGenerationError::from_model_value)?,
        }),
        RuntimeRequirementDto::BooleanOptionEnabled { option } => {
            Ok(RuntimeRequirement::BooleanOptionEnabled {
                option: RuntimeBooleanOption::try_from(option.as_str())
                    .map_err(CapabilityGenerationError::from_model_value)?,
            })
        }
    }
}

fn parse_message_subject(
    method: &Definition,
    dto: MessageSubjectDto,
) -> Result<MessageSubjectRef, CapabilityGenerationError> {
    match dto {
        MessageSubjectDto::One {
            chat_argument,
            message_argument,
        } => Ok(MessageSubjectRef::One {
            chat: parse_chat_target(method, chat_argument)?,
            message: MessageIdRef::try_from(parse_role_argument(
                method,
                message_argument,
                &["message_id"],
                "int53",
            )?)
            .map_err(CapabilityGenerationError::from_model_value)?,
        }),
        MessageSubjectDto::Each {
            chat_argument,
            message_argument,
        } => {
            let messages = parse_argument(method, message_argument)?;
            if messages.as_str() != "message_ids" {
                return Err(CapabilityGenerationError::invalid_policy(format!(
                    "method {:?} multi-message requirement needs semantic message_ids argument",
                    method.name()
                )));
            }
            require_vector_argument_type(method, &messages, "int53")?;
            Ok(MessageSubjectRef::Each {
                chat: parse_chat_target(method, chat_argument)?,
                messages: MessageIdsRef::try_from(messages)
                    .map_err(CapabilityGenerationError::from_model_value)?,
            })
        }
    }
}

fn parse_group_call_id(
    method: &Definition,
    argument: String,
) -> Result<GroupCallIdRef, CapabilityGenerationError> {
    GroupCallIdRef::try_from(parse_role_argument(
        method,
        argument,
        &["group_call_id"],
        "int32",
    )?)
    .map_err(CapabilityGenerationError::from_model_value)
}

fn parse_group_call_message_subject(
    method: &Definition,
    dto: GroupCallMessageSubjectDto,
) -> Result<GroupCallMessageSubjectRef, CapabilityGenerationError> {
    match dto {
        GroupCallMessageSubjectDto::Each {
            group_call_argument,
            message_argument,
        } => {
            let messages = parse_argument(method, message_argument)?;
            if messages.as_str() != "message_ids" {
                return Err(CapabilityGenerationError::invalid_policy(format!(
                    "method {:?} group-call message requirement needs semantic message_ids argument",
                    method.name()
                )));
            }
            require_vector_argument_type(method, &messages, "int32")?;
            Ok(GroupCallMessageSubjectRef::Each {
                group_call: parse_group_call_id(method, group_call_argument)?,
                messages: GroupCallMessageIdsRef::try_from(messages)
                    .map_err(CapabilityGenerationError::from_model_value)?,
            })
        }
    }
}

fn parse_parameter_notices(
    dtos: Vec<ParameterNoticeDto>,
    method: &Definition,
) -> Result<Vec<ParameterCapabilityNotice>, CapabilityGenerationError> {
    if dtos.len() > MAX_PARAMETER_NOTICES_PER_METHOD {
        return Err(CapabilityGenerationError::resource_limit(format!(
            "parameter notices for {:?} exceed the {MAX_PARAMETER_NOTICES_PER_METHOD}-notice cap",
            method.name()
        )));
    }
    dtos.into_iter()
        .map(|dto| {
            let parameter = parse_argument(method, dto.parameter)?;
            let gate = match dto.gate {
                ParameterGateDto::Account(value) => ParameterGate::Account(
                    AccountKind::try_from(value.as_str())
                        .map_err(CapabilityGenerationError::from_model_value)?,
                ),
                ParameterGateDto::CurrentAccountEntitlement(value) => {
                    ParameterGate::CurrentAccountEntitlement(
                        CurrentAccountEntitlement::try_from(value.as_str())
                            .map_err(CapabilityGenerationError::from_model_value)?,
                    )
                }
                ParameterGateDto::Application(value) => ParameterGate::Application(
                    ApplicationRequirement::try_from(value.as_str())
                        .map_err(CapabilityGenerationError::from_model_value)?,
                ),
                ParameterGateDto::DcEnvironment(value) => ParameterGate::DcEnvironment(
                    DcEnvironment::try_from(value.as_str())
                        .map_err(CapabilityGenerationError::from_model_value)?,
                ),
            };
            ParameterCapabilityNotice::try_new(parameter, gate)
                .map_err(|error| CapabilityGenerationError::invalid_policy(error.to_string()))
        })
        .collect()
}

fn parse_values<T, E>(
    name: &str,
    values: Vec<String>,
    max_values: usize,
    parser: impl Fn(&str) -> Result<T, E>,
) -> Result<Vec<T>, CapabilityGenerationError>
where
    E: fmt::Display,
{
    if values.len() > max_values {
        return Err(CapabilityGenerationError::resource_limit(format!(
            "{name} has {} values, exceeding the {max_values}-value cap",
            values.len()
        )));
    }
    values
        .iter()
        .map(|value| parser(value).map_err(CapabilityGenerationError::from_model_value))
        .collect()
}

fn parse_argument(
    method: &Definition,
    value: String,
) -> Result<ArgumentRef, CapabilityGenerationError> {
    let argument = ArgumentRef::try_from(value.as_str())
        .map_err(|error| CapabilityGenerationError::invalid_policy(error.to_string()))?;
    if field_type(method, argument.as_str()).is_none() {
        return Err(CapabilityGenerationError::invalid_policy(format!(
            "method {:?} has no named field {:?}",
            method.name(),
            argument.as_str()
        )));
    }
    Ok(argument)
}

fn parse_role_argument(
    method: &Definition,
    value: String,
    allowed_names: &[&str],
    expected_type: &str,
) -> Result<ArgumentRef, CapabilityGenerationError> {
    let argument = parse_argument(method, value)?;
    if !allowed_names.contains(&argument.as_str()) {
        return Err(CapabilityGenerationError::invalid_policy(format!(
            "method {:?} requirement needs semantic argument from {allowed_names:?}, got {:?}",
            method.name(),
            argument.as_str()
        )));
    }
    require_argument_type(method, &argument, expected_type)?;
    Ok(argument)
}

fn parse_chat_target(
    method: &Definition,
    value: String,
) -> Result<ChatTargetRef, CapabilityGenerationError> {
    let argument = parse_argument(method, value)?;
    require_argument_type(method, &argument, "int53")?;
    ChatTargetRef::try_from(argument).map_err(|error| {
        CapabilityGenerationError::invalid_policy(format!(
            "method {:?} has invalid chat-level target: {error}",
            method.name()
        ))
    })
}

fn require_argument_type(
    method: &Definition,
    argument: &ArgumentRef,
    expected_type: &str,
) -> Result<(), CapabilityGenerationError> {
    let ty = field_type(method, argument.as_str()).ok_or_else(|| {
        CapabilityGenerationError::invalid_policy(format!(
            "method {:?} has no named field {:?}",
            method.name(),
            argument.as_str()
        ))
    })?;
    if ty.name() != expected_type || !ty.arguments().is_empty() {
        return Err(CapabilityGenerationError::invalid_policy(format!(
            "method {:?} field {:?} must have exact type {expected_type}",
            method.name(),
            argument.as_str()
        )));
    }
    Ok(())
}

fn require_vector_argument_type(
    method: &Definition,
    argument: &ArgumentRef,
    element_type: &str,
) -> Result<(), CapabilityGenerationError> {
    let ty = field_type(method, argument.as_str()).ok_or_else(|| {
        CapabilityGenerationError::invalid_policy(format!(
            "method {:?} has no named field {:?}",
            method.name(),
            argument.as_str()
        ))
    })?;
    let exact = ty.name() == "vector"
        && matches!(ty.arguments(), [element] if element.name() == element_type && element.arguments().is_empty());
    if !exact {
        return Err(CapabilityGenerationError::invalid_policy(format!(
            "method {:?} field {:?} must have exact type vector<{element_type}>",
            method.name(),
            argument.as_str()
        )));
    }
    Ok(())
}

fn field_type<'a>(
    method: &'a Definition,
    argument: &str,
) -> Option<&'a telegram_core::schema::TypeRef> {
    method
        .parameters()
        .iter()
        .find_map(|parameter| match parameter {
            Parameter::Field { name, ty } if name == argument => Some(ty),
            _ => None,
        })
}

fn validate_authorization_states(schema: &Schema) -> Result<(), CapabilityGenerationError> {
    let actual = schema
        .inventory()
        .authorization_state_names()
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let expected = AuthorizationState::ALL
        .iter()
        .map(|state| state.as_str())
        .collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            "schema authorization-state inventory differs from the exact pinned 13-state vocabulary",
        ));
    }
    Ok(())
}

fn validate_right_vocabularies(schema: &Schema) -> Result<(), CapabilityGenerationError> {
    let mut administrator = ChatAdministratorRight::ALL
        .iter()
        .map(|right| right.as_str())
        .collect::<BTreeSet<_>>();
    administrator.insert("is_anonymous");
    validate_bool_constructor(schema, "chatAdministratorRights", administrator)?;
    validate_bool_constructor(
        schema,
        "chatPermissions",
        ChatMemberRight::ALL
            .iter()
            .map(|right| right.as_str())
            .collect(),
    )?;
    validate_bool_constructor(
        schema,
        "businessBotRights",
        BusinessBotRight::ALL
            .iter()
            .map(|right| right.as_str())
            .collect(),
    )
}

fn validate_chat_type_vocabulary(schema: &Schema) -> Result<(), CapabilityGenerationError> {
    let expected = BTreeSet::from([
        "chatTypePrivate user_id:int53 = ChatType;",
        "chatTypeBasicGroup basic_group_id:int53 = ChatType;",
        "chatTypeSupergroup supergroup_id:int53 is_channel:Bool = ChatType;",
        "chatTypeSecret secret_chat_id:int32 user_id:int53 = ChatType;",
    ]);
    let actual = schema
        .definitions()
        .iter()
        .filter(|definition| {
            definition.kind() == DefinitionKind::Constructor
                && definition.result().name() == "ChatType"
        })
        .map(|definition| definition.canonical_signature())
        .collect::<BTreeSet<_>>();

    if actual != expected {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            "schema ChatType constructors differ from the exact pinned four-constructor shapes",
        ));
    }
    Ok(())
}

fn validate_message_properties_vocabulary(
    schema: &Schema,
) -> Result<(), CapabilityGenerationError> {
    let mut expected_fields = MessageCapability::ALL
        .iter()
        .map(|capability| capability.as_str())
        .collect::<Vec<_>>();
    expected_fields.extend([
        "has_protected_content_by_current_user",
        "has_protected_content_by_other_user",
        "need_show_statistics",
    ]);
    let expected_constructor = format!(
        "messageProperties {} = MessageProperties;",
        expected_fields
            .into_iter()
            .map(|field| format!("{field}:Bool"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let constructors = schema
        .definitions()
        .iter()
        .filter(|definition| {
            definition.kind() == DefinitionKind::Constructor
                && definition.result().name() == "MessageProperties"
        })
        .collect::<Vec<_>>();
    if !matches!(
        constructors.as_slice(),
        [constructor] if constructor.canonical_signature() == expected_constructor
    ) {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            "schema MessageProperties constructors differ from the exact pinned shape",
        ));
    }

    let methods = schema
        .methods()
        .iter()
        .filter(|method| method.name() == "getMessageProperties")
        .collect::<Vec<_>>();
    if !matches!(
        methods.as_slice(),
        [method]
            if method.canonical_signature()
                == "getMessageProperties chat_id:int53 message_id:int53 = MessageProperties;"
    ) {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            "schema getMessageProperties method differs from the exact pinned signature",
        ));
    }
    Ok(())
}

fn validate_group_call_vocabulary(schema: &Schema) -> Result<(), CapabilityGenerationError> {
    const GROUP_CALL: &str = "groupCall id:int32 unique_id:int64 title:string invite_link:string paid_message_star_count:int53 scheduled_start_date:int32 enabled_start_notification:Bool is_active:Bool is_video_chat:Bool is_live_story:Bool is_rtmp_stream:Bool is_joined:Bool need_rejoin:Bool is_owned:Bool can_be_managed:Bool participant_count:int32 has_hidden_listeners:Bool loaded_all_participants:Bool message_sender_id:MessageSender recent_speakers:vector<groupCallRecentSpeaker> is_my_video_enabled:Bool is_my_video_paused:Bool can_enable_video:Bool mute_new_participants:Bool can_toggle_mute_new_participants:Bool can_send_messages:Bool are_messages_allowed:Bool can_toggle_are_messages_allowed:Bool can_delete_messages:Bool record_duration:int32 is_video_recorded:Bool duration:int32 = GroupCall;";
    const GROUP_CALL_MESSAGE: &str = "groupCallMessage message_id:int32 sender_id:MessageSender date:int32 text:formattedText paid_message_star_count:int53 is_from_owner:Bool can_be_deleted:Bool = GroupCallMessage;";
    const GET_GROUP_CALL: &str = "getGroupCall group_call_id:int32 = GroupCall;";
    const UPDATE_GROUP_CALL: &str = "updateGroupCall group_call:groupCall = Update;";
    const UPDATE_NEW_GROUP_CALL_MESSAGE: &str =
        "updateNewGroupCallMessage group_call_id:int32 message:groupCallMessage = Update;";
    const UPDATE_GROUP_CALL_MESSAGES_DELETED: &str =
        "updateGroupCallMessagesDeleted group_call_id:int32 message_ids:vector<int32> = Update;";

    for (result, expected, label) in [
        ("GroupCall", GROUP_CALL, "GroupCall"),
        ("GroupCallMessage", GROUP_CALL_MESSAGE, "GroupCallMessage"),
    ] {
        let constructors = schema
            .definitions()
            .iter()
            .filter(|definition| {
                definition.kind() == DefinitionKind::Constructor
                    && definition.result().name() == result
            })
            .collect::<Vec<_>>();
        if !matches!(constructors.as_slice(), [constructor] if constructor.canonical_signature() == expected)
        {
            return Err(CapabilityGenerationError::new(
                CapabilityGenerationErrorKind::SchemaDrift,
                format!("schema {label} constructor differs from the exact pinned shape"),
            ));
        }
    }

    let methods = schema
        .methods()
        .iter()
        .filter(|method| method.name() == "getGroupCall")
        .collect::<Vec<_>>();
    if !matches!(methods.as_slice(), [method] if method.canonical_signature() == GET_GROUP_CALL) {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            "schema getGroupCall method differs from the exact pinned signature",
        ));
    }

    for (name, expected) in [
        ("updateGroupCall", UPDATE_GROUP_CALL),
        ("updateNewGroupCallMessage", UPDATE_NEW_GROUP_CALL_MESSAGE),
        (
            "updateGroupCallMessagesDeleted",
            UPDATE_GROUP_CALL_MESSAGES_DELETED,
        ),
    ] {
        let updates = schema
            .definitions()
            .iter()
            .filter(|definition| definition.name() == name)
            .collect::<Vec<_>>();
        if !matches!(updates.as_slice(), [update] if update.canonical_signature() == expected) {
            return Err(CapabilityGenerationError::new(
                CapabilityGenerationErrorKind::SchemaDrift,
                format!("schema {name} differs from the exact pinned signature"),
            ));
        }
    }
    Ok(())
}

fn validate_supergroup_vocabulary(schema: &Schema) -> Result<(), CapabilityGenerationError> {
    const SUPERGROUP: &str = "supergroup id:int53 usernames:usernames date:int32 status:ChatMemberStatus member_count:int32 boost_level:int32 has_automatic_translation:Bool has_linked_chat:Bool has_location:Bool sign_messages:Bool show_message_sender:Bool join_to_send_messages:Bool join_by_request:Bool is_slow_mode_enabled:Bool is_channel:Bool is_broadcast_group:Bool is_forum:Bool is_direct_messages_group:Bool is_administered_direct_messages_group:Bool verification_status:verificationStatus has_direct_messages_group:Bool has_forum_tabs:Bool restriction_info:restrictionInfo paid_message_star_count:int53 active_story_state:ActiveStoryState = Supergroup;";
    const UPDATE_SUPERGROUP: &str = "updateSupergroup supergroup:supergroup = Update;";

    let constructors = schema
        .definitions()
        .iter()
        .filter(|definition| {
            definition.kind() == DefinitionKind::Constructor
                && definition.result().name() == "Supergroup"
        })
        .collect::<Vec<_>>();
    if !matches!(constructors.as_slice(), [constructor] if constructor.canonical_signature() == SUPERGROUP)
    {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            "schema Supergroup constructor differs from the exact pinned shape",
        ));
    }

    let updates = schema
        .definitions()
        .iter()
        .filter(|definition| definition.name() == "updateSupergroup")
        .collect::<Vec<_>>();
    if !matches!(updates.as_slice(), [update] if update.canonical_signature() == UPDATE_SUPERGROUP)
    {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            "schema updateSupergroup differs from the exact pinned signature",
        ));
    }
    Ok(())
}

fn validate_supergroup_full_info_vocabulary(
    schema: &Schema,
) -> Result<(), CapabilityGenerationError> {
    const SUPERGROUP_FULL_INFO: &str = "supergroupFullInfo photo:chatPhoto community_id:int53 description:string member_count:int32 administrator_count:int32 restricted_count:int32 banned_count:int32 linked_chat_id:int53 direct_messages_chat_id:int53 slow_mode_delay:int32 slow_mode_delay_expires_in:double can_enable_paid_messages:Bool can_enable_paid_reaction:Bool can_get_members:Bool has_hidden_members:Bool can_hide_members:Bool can_set_sticker_set:Bool can_set_location:Bool can_get_statistics:Bool can_get_revenue_statistics:Bool can_get_star_revenue_statistics:Bool can_send_gift:Bool can_toggle_aggressive_anti_spam:Bool is_all_history_available:Bool can_have_sponsored_messages:Bool has_aggressive_anti_spam_enabled:Bool has_paid_media_allowed:Bool has_pinned_stories:Bool gift_count:int32 my_boost_count:int32 unrestrict_boost_count:int32 outgoing_paid_message_star_count:int53 sticker_set_id:int64 custom_emoji_sticker_set_id:int64 location:chatLocation invite_link:chatInviteLink guard_bot_user_id:int53 bot_commands:vector<botCommands> bot_verification:botVerification main_profile_tab:ProfileTab upgraded_from_basic_group_id:int53 upgraded_from_max_message_id:int53 = SupergroupFullInfo;";
    const GET_SUPERGROUP_FULL_INFO: &str =
        "getSupergroupFullInfo supergroup_id:int53 = SupergroupFullInfo;";
    const UPDATE_SUPERGROUP_FULL_INFO: &str = "updateSupergroupFullInfo supergroup_id:int53 supergroup_full_info:supergroupFullInfo = Update;";

    let constructors = schema
        .definitions()
        .iter()
        .filter(|definition| {
            definition.kind() == DefinitionKind::Constructor
                && definition.result().name() == "SupergroupFullInfo"
        })
        .collect::<Vec<_>>();
    if !matches!(constructors.as_slice(), [constructor] if constructor.canonical_signature() == SUPERGROUP_FULL_INFO)
    {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            "schema SupergroupFullInfo constructor differs from the exact pinned shape",
        ));
    }

    let getters = schema
        .methods()
        .iter()
        .filter(|method| method.name() == "getSupergroupFullInfo")
        .collect::<Vec<_>>();
    if !matches!(getters.as_slice(), [getter] if getter.canonical_signature() == GET_SUPERGROUP_FULL_INFO)
    {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            "schema getSupergroupFullInfo method differs from the exact pinned signature",
        ));
    }

    let updates = schema
        .definitions()
        .iter()
        .filter(|definition| definition.name() == "updateSupergroupFullInfo")
        .collect::<Vec<_>>();
    if !matches!(updates.as_slice(), [update] if update.canonical_signature() == UPDATE_SUPERGROUP_FULL_INFO)
    {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            "schema updateSupergroupFullInfo differs from the exact pinned signature",
        ));
    }
    Ok(())
}

fn validate_runtime_boolean_option_vocabulary(
    schema: &Schema,
) -> Result<(), CapabilityGenerationError> {
    const OPTION_VALUES: [&str; 4] = [
        "optionValueBoolean value:Bool = OptionValue;",
        "optionValueEmpty = OptionValue;",
        "optionValueInteger value:int64 = OptionValue;",
        "optionValueString value:string = OptionValue;",
    ];
    const GET_OPTION: &str = "getOption name:string = OptionValue;";
    const UPDATE_OPTION: &str = "updateOption name:string value:OptionValue = Update;";

    let actual = schema
        .definitions()
        .iter()
        .filter(|definition| {
            definition.kind() == DefinitionKind::Constructor
                && definition.result().name() == "OptionValue"
        })
        .map(Definition::canonical_signature)
        .collect::<BTreeSet<_>>();
    let expected = OPTION_VALUES.into_iter().collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            "schema OptionValue constructors differ from the exact pinned vocabulary",
        ));
    }

    let getters = schema
        .methods()
        .iter()
        .filter(|method| method.name() == "getOption")
        .collect::<Vec<_>>();
    if !matches!(getters.as_slice(), [getter] if getter.canonical_signature() == GET_OPTION) {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            "schema getOption method differs from the exact pinned signature",
        ));
    }

    let updates = schema
        .definitions()
        .iter()
        .filter(|definition| definition.name() == "updateOption")
        .collect::<Vec<_>>();
    if !matches!(updates.as_slice(), [update] if update.kind() == DefinitionKind::Constructor && update.canonical_signature() == UPDATE_OPTION)
    {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            "schema updateOption differs from the exact pinned signature",
        ));
    }
    Ok(())
}

fn validate_bool_constructor(
    schema: &Schema,
    constructor_name: &str,
    expected_fields: BTreeSet<&str>,
) -> Result<(), CapabilityGenerationError> {
    let constructor = schema
        .definitions()
        .iter()
        .find(|definition| {
            definition.kind() == DefinitionKind::Constructor
                && definition.name() == constructor_name
        })
        .ok_or_else(|| {
            CapabilityGenerationError::new(
                CapabilityGenerationErrorKind::SchemaDrift,
                format!("schema is missing exact {constructor_name} constructor"),
            )
        })?;
    let mut actual_fields = BTreeSet::new();
    for parameter in constructor.parameters() {
        let Parameter::Field { name, ty } = parameter else {
            return Err(CapabilityGenerationError::new(
                CapabilityGenerationErrorKind::SchemaDrift,
                format!("{constructor_name} contains a non-field parameter"),
            ));
        };
        if ty.name() != "Bool" || !ty.arguments().is_empty() {
            return Err(CapabilityGenerationError::new(
                CapabilityGenerationErrorKind::SchemaDrift,
                format!("{constructor_name}.{name} is not exact Bool"),
            ));
        }
        actual_fields.insert(name.as_str());
    }
    if actual_fields != expected_fields {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            format!(
                "schema fields for {constructor_name} differ from the closed rights vocabulary"
            ),
        ));
    }
    Ok(())
}

fn validate_documented_authorization_states(
    method: &Definition,
    descriptor: &CapabilityDescriptor,
) -> Result<(), CapabilityGenerationError> {
    let documentation = method_documentation_text(method);
    let normalized = documentation.to_ascii_lowercase();
    let mentioned = AuthorizationState::ALL
        .iter()
        .copied()
        .filter(|state| documentation.contains(state.as_str()))
        .collect::<BTreeSet<_>>();
    let expected = if method.name() == "destroy"
        && normalized.contains("can be called before authorization")
    {
        Some(post_initialization_authorization_states())
    } else if normalized.contains("works only when the current authorization state is") {
        if mentioned.is_empty() {
            return Err(CapabilityGenerationError::new(
                CapabilityGenerationErrorKind::SchemaDrift,
                format!(
                    "authorization-state contract for {:?} names no known state",
                    method.name()
                ),
            ));
        }
        Some(mentioned)
    } else if normalized.contains("can be called before authorization when") {
        let mut states = mentioned;
        states.insert(AuthorizationState::Ready);
        Some(states)
    } else if normalized.contains("can be called before authorization")
        || normalized.contains("can be called before initialization")
    {
        Some(requestable_authorization_states())
    } else if method.name() == "getAuthenticationPasskeyParameters" {
        // TDLib documents this method through the authorization-state
        // cross-reference and the adjacent checkAuthenticationPasskey contract,
        // rather than on the method itself. Keep this reviewed exception exact;
        // arbitrary methods remain Ready-only.
        Some(passkey_authorization_states())
    } else {
        Some(BTreeSet::from([AuthorizationState::Ready]))
    };
    let actual = descriptor
        .authorization_states()
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let contradicts = expected
        .as_ref()
        .is_some_and(|expected| actual != *expected);
    if contradicts {
        return Err(CapabilityGenerationError::invalid_policy(format!(
            "authorization states for {:?} contradict method documentation",
            method.name()
        )));
    }
    Ok(())
}

fn passkey_authorization_states() -> BTreeSet<AuthorizationState> {
    BTreeSet::from([
        AuthorizationState::WaitPhoneNumber,
        AuthorizationState::WaitPremiumPurchase,
        AuthorizationState::WaitEmailAddress,
        AuthorizationState::WaitEmailCode,
        AuthorizationState::WaitCode,
        AuthorizationState::WaitOtherDeviceConfirmation,
        AuthorizationState::WaitRegistration,
        AuthorizationState::WaitPassword,
    ])
}

fn requestable_authorization_states() -> BTreeSet<AuthorizationState> {
    BTreeSet::from([
        AuthorizationState::WaitTdlibParameters,
        AuthorizationState::WaitPhoneNumber,
        AuthorizationState::WaitPremiumPurchase,
        AuthorizationState::WaitEmailAddress,
        AuthorizationState::WaitEmailCode,
        AuthorizationState::WaitCode,
        AuthorizationState::WaitOtherDeviceConfirmation,
        AuthorizationState::WaitRegistration,
        AuthorizationState::WaitPassword,
        AuthorizationState::Ready,
    ])
}

fn post_initialization_authorization_states() -> BTreeSet<AuthorizationState> {
    let mut states = requestable_authorization_states();
    states.remove(&AuthorizationState::WaitTdlibParameters);
    states
}

fn validate_synchronous_documentation(
    method: &Definition,
    capability: &SynchronousCapability,
) -> Result<(), CapabilityGenerationError> {
    let description = method_description(method).to_ascii_lowercase();
    let documented = description.contains("can be called synchronously");
    let conditional = description.contains("can be called synchronously for");
    let valid = match capability {
        SynchronousCapability::Never => !documented,
        SynchronousCapability::Always => documented && !conditional,
        SynchronousCapability::StringParameterValues(condition) => {
            if !conditional {
                false
            } else {
                let documented_values = quoted_values(&description);
                !documented_values.is_empty()
                    && condition
                        .values()
                        .iter()
                        .map(ParameterStringValue::as_str)
                        .collect::<BTreeSet<_>>()
                        == documented_values
            }
        }
    };
    if !valid {
        return Err(CapabilityGenerationError::invalid_policy(format!(
            "synchronous capability for {:?} contradicts @description",
            method.name()
        )));
    }
    Ok(())
}

fn validate_documented_method_constraints(
    method: &Definition,
    descriptor: &CapabilityDescriptor,
) -> Result<(), CapabilityGenerationError> {
    let description = method_description(method).to_ascii_lowercase();
    let runtime_contract = reviewed_runtime_contract(method.name(), &normalized_text(&description));
    let group_call_contract = reviewed_group_call_contract(method)?;
    let message_moderation_contract = reviewed_message_moderation_contract(method)?;
    let supergroup_full_info_contract = reviewed_supergroup_full_info_contract(method)?;
    let boolean_option_contract = reviewed_runtime_boolean_option_contract(method)?;
    let chat_boost_contract = reviewed_chat_boost_contract(method)?;
    let chat_event_log_contract = reviewed_chat_event_log_contract(method)?;
    let chat_invite_link_contract = reviewed_chat_invite_link_contract(method)?;
    let chat_setting_contract = reviewed_chat_setting_contract(method)?;
    let supergroup_username_contract = reviewed_supergroup_username_contract(method)?;
    let video_chat_contract = reviewed_video_chat_contract(method)?;
    let ready = descriptor.ready_accounts();
    let entitlements = descriptor.current_account_entitlements();
    let dcs = descriptor.dc_environments();
    let application = descriptor.application();

    let bot_only = description.contains("for bots only");
    let regular_only = description.contains("for regular users only")
        || description.contains("current users only");
    let premium_only = description.contains("for telegram premium users only")
        || description.contains("current premium users only");
    let business_only = description.contains("requires telegram business subscription")
        || description.contains("current business users only");
    let official_only = description.contains("for official telegram apps only")
        || description.contains("official applications only");
    let expected_entitlements = [
        premium_only.then_some(CurrentAccountEntitlement::Premium),
        business_only.then_some(CurrentAccountEntitlement::Business),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let allows_ready = descriptor
        .authorization_states()
        .contains(&AuthorizationState::Ready);
    let runtime_bot_only = matches!(
        runtime_contract,
        Some(ReviewedRuntimeContract::BusinessConnectionEnabledAndRight(
            _
        ))
    );
    let runtime_regular_only = matches!(
        runtime_contract,
        Some(ReviewedRuntimeContract::OwnerInKind(_))
    ) || group_call_contract.is_some()
        || message_moderation_contract.is_some_and(|contract| contract.regular_user_only())
        || supergroup_full_info_contract.is_some()
        || boolean_option_contract.is_some_and(|contract| contract.regular_user_only)
        || chat_boost_contract.is_some()
        || chat_event_log_contract.is_some()
        || chat_invite_link_contract.is_some_and(|contract| contract.regular_user_only())
        || chat_setting_contract.is_some_and(|contract| contract.regular_user_only())
        || supergroup_username_contract.is_some()
        || video_chat_contract.is_some();
    let expected_ready = if bot_only || runtime_bot_only {
        vec![AccountKind::Bot]
    } else if regular_only || runtime_regular_only || !expected_entitlements.is_empty() {
        vec![AccountKind::RegularUser]
    } else {
        vec![AccountKind::RegularUser, AccountKind::Bot]
    };
    let expected_application = if description.contains("official mobile applications only") {
        ApplicationRequirement::OfficialMobile
    } else if official_only {
        ApplicationRequirement::Official
    } else {
        ApplicationRequirement::Any
    };
    let expected_dcs = if description.contains("test dc only") {
        vec![DcEnvironment::Test]
    } else if description.contains("production dc only") {
        vec![DcEnvironment::Production]
    } else {
        vec![DcEnvironment::Production, DcEnvironment::Test]
    };
    let valid = (!allows_ready || ready == expected_ready)
        && entitlements == expected_entitlements
        && application == expected_application
        && dcs == expected_dcs;
    if !valid {
        return Err(CapabilityGenerationError::invalid_policy(format!(
            "method-level capability for {:?} contradicts @description",
            method.name()
        )));
    }
    Ok(())
}

fn validate_documented_runtime_requirements(
    method: &Definition,
    descriptor: &CapabilityDescriptor,
) -> Result<(), CapabilityGenerationError> {
    let expected =
        documented_runtime_requirements(method)?.unwrap_or_else(RequirementAlternatives::always);
    if descriptor.runtime_requirements() != &expected {
        return Err(CapabilityGenerationError::invalid_policy(format!(
            "runtime requirements for {:?} contradict @description",
            method.name()
        )));
    }
    Ok(())
}

fn documented_runtime_requirements(
    method: &Definition,
) -> Result<Option<RequirementAlternatives>, CapabilityGenerationError> {
    let description = normalized_text(&method_description(method));
    let runtime_contract = reviewed_runtime_contract(method.name(), &description);
    let message_contract = reviewed_message_capability_contract(method)?;
    let group_call_contract = reviewed_group_call_contract(method)?;
    let message_moderation_contract = reviewed_message_moderation_contract(method)?;
    let supergroup_full_info_contract = reviewed_supergroup_full_info_contract(method)?;
    let boolean_option_contract = reviewed_runtime_boolean_option_contract(method)?;
    let chat_boost_contract = reviewed_chat_boost_contract(method)?;
    let chat_event_log_contract = reviewed_chat_event_log_contract(method)?;
    let chat_invite_link_contract = reviewed_chat_invite_link_contract(method)?;
    let chat_setting_contract = reviewed_chat_setting_contract(method)?;
    let supergroup_username_contract = reviewed_supergroup_username_contract(method)?;
    let video_chat_contract = reviewed_video_chat_contract(method)?;
    let reviewed_family_count = [
        runtime_contract.is_some(),
        message_contract.is_some(),
        group_call_contract.is_some(),
        message_moderation_contract.is_some(),
        supergroup_full_info_contract.is_some(),
        boolean_option_contract.is_some(),
        chat_boost_contract.is_some(),
        chat_event_log_contract.is_some(),
        chat_invite_link_contract.is_some(),
        chat_setting_contract.is_some(),
        supergroup_username_contract.is_some(),
        video_chat_contract.is_some(),
    ]
    .into_iter()
    .filter(|present| *present)
    .count();
    if reviewed_family_count > 1 {
        return Err(unsupported_runtime_documentation(
            method,
            "multiple reviewed runtime contract families overlap",
        ));
    }
    let dispositions = documented_runtime_signal_dispositions(method)?;
    if dispositions
        .iter()
        .any(|(_, disposition)| matches!(disposition, RuntimeSignalDisposition::Deferred(_)))
    {
        return Err(unsupported_runtime_documentation(
            method,
            "at least one runtime signal still needs a typed disposition",
        ));
    }

    if reviewed_family_count == 0 {
        if dispositions.iter().any(|(_, disposition)| {
            *disposition == RuntimeSignalDisposition::ConsumedByRuntimeRequirements
        }) {
            return Err(unsupported_runtime_documentation(
                method,
                "a consumed runtime signal has no reviewed requirement contract",
            ));
        }
        return Ok(None);
    }
    let consumed = dispositions
        .iter()
        .filter_map(|(key, disposition)| {
            (*disposition == RuntimeSignalDisposition::ConsumedByRuntimeRequirements)
                .then_some(key.clone())
        })
        .collect::<BTreeSet<_>>();
    let mut expected_consumed = runtime_contract
        .map(ReviewedRuntimeContract::consumed_signal_keys)
        .unwrap_or_default();
    if let Some(contract) = message_contract {
        expected_consumed.extend(contract.consumed_signal_keys()?);
    }
    if let Some(contract) = group_call_contract {
        expected_consumed.extend(contract.consumed_signal_keys());
    }
    if message_moderation_contract.is_some() {
        expected_consumed.extend(message_moderation_consumed_signal_keys());
    }
    if let Some(contract) = supergroup_full_info_contract {
        expected_consumed.extend(contract.consumed_signal_keys());
    }
    if let Some(contract) = boolean_option_contract {
        expected_consumed.extend(contract.consumed_signal_keys());
    }
    if chat_boost_contract.is_some() {
        expected_consumed.extend(chat_administrator_consumed_signal_keys());
    }
    if chat_event_log_contract.is_some() {
        expected_consumed.extend(chat_administrator_consumed_signal_keys());
    }
    if let Some(contract) = chat_invite_link_contract {
        expected_consumed.extend(chat_invite_link_consumed_signal_keys(contract));
    }
    if let Some(contract) = chat_setting_contract {
        expected_consumed.extend(chat_setting_consumed_signal_keys(contract));
    }
    if supergroup_username_contract.is_some() {
        expected_consumed.extend(supergroup_username_consumed_signal_keys());
    }
    if let Some(contract) = video_chat_contract {
        expected_consumed.extend(video_chat_consumed_signal_keys(contract));
    }
    if consumed != expected_consumed {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed runtime requirements don't consume their exact signal set",
        ));
    }
    let clauses = if let Some(contract) = message_contract {
        documented_message_capability_clauses(method, contract)?
    } else if let Some(contract) = group_call_contract {
        documented_group_call_clauses(method, contract)?
    } else if let Some(contract) = message_moderation_contract {
        documented_message_moderation_clauses(method, contract)?
    } else if let Some(contract) = supergroup_full_info_contract {
        documented_supergroup_full_info_clauses(method, contract)?
    } else if let Some(contract) = boolean_option_contract {
        vec![vec![RuntimeRequirement::BooleanOptionEnabled {
            option: contract.option,
        }]]
    } else if chat_boost_contract.is_some() {
        let target = documented_chat_target(method)?;
        vec![vec![RuntimeRequirement::ChatAdministrator { target }]]
    } else if let Some(contract) = chat_event_log_contract {
        let target = documented_chat_target(method)?;
        chat_kind_clauses(&target, contract.supported_chat_kinds(), |target| {
            RuntimeRequirement::ChatAdministrator {
                target: target.clone(),
            }
        })?
    } else if let Some(contract) = supergroup_username_contract {
        let target = documented_chat_target(method)?;
        chat_kind_clauses(&target, contract.supported_chat_kinds(), |target| {
            RuntimeRequirement::ChatOwner {
                target: target.clone(),
            }
        })?
    } else if let Some(contract) = chat_invite_link_contract {
        let target = documented_chat_target(method)?;
        match contract.required_access() {
            chat_invite_links::RequiredAccess::AdministratorRight(right) => {
                chat_kind_clauses(&target, contract.supported_chat_kinds(), |target| {
                    RuntimeRequirement::ChatAdministratorRight {
                        target: target.clone(),
                        right,
                    }
                })?
            }
            chat_invite_links::RequiredAccess::Owner => {
                chat_kind_clauses(&target, contract.supported_chat_kinds(), |target| {
                    RuntimeRequirement::ChatOwner {
                        target: target.clone(),
                    }
                })?
            }
        }
    } else if let Some(contract) = chat_setting_contract {
        documented_chat_setting_clauses(method, contract)?
    } else if let Some(contract) = video_chat_contract {
        let target = documented_chat_target(method)?;
        chat_kind_clauses(
            &target,
            contract.supported_chat_kinds(),
            |target| match contract.required_access() {
                video_chats::RequiredAccess::AdministratorRight(right) => {
                    RuntimeRequirement::ChatAdministratorRight {
                        target: target.clone(),
                        right,
                    }
                }
                video_chats::RequiredAccess::Owner => RuntimeRequirement::ChatOwner {
                    target: target.clone(),
                },
            },
        )?
    } else {
        let Some(contract) = runtime_contract else {
            return Err(unsupported_runtime_documentation(
                method,
                "reviewed runtime contract disappeared during validation",
            ));
        };
        match contract {
            ReviewedRuntimeContract::AdministratorInKinds(kinds) => {
                let target = documented_chat_target(method)?;
                chat_kind_clauses(&target, kinds, |target| {
                    RuntimeRequirement::ChatAdministrator {
                        target: target.clone(),
                    }
                })?
            }
            ReviewedRuntimeContract::AdministratorRightInKinds { right, kinds } => {
                let target = documented_chat_target(method)?;
                chat_kind_clauses(&target, kinds, |target| {
                    RuntimeRequirement::ChatAdministratorRight {
                        target: target.clone(),
                        right,
                    }
                })?
            }
            ReviewedRuntimeContract::OwnerInKind(kind) => {
                let target = documented_chat_target(method)?;
                vec![vec![
                    documented_chat_kind(&target, kind)?,
                    RuntimeRequirement::ChatOwner { target },
                ]]
            }
            ReviewedRuntimeContract::AdministratorOrTopicCreator { right, kind } => {
                let target = documented_chat_target(method)?;
                let chat_kind = documented_chat_kind(&target, kind)?;
                vec![
                    vec![
                        chat_kind.clone(),
                        RuntimeRequirement::ChatAdministratorRight {
                            target: target.clone(),
                            right,
                        },
                    ],
                    vec![
                        chat_kind,
                        RuntimeRequirement::TopicCreator {
                            target,
                            topic: documented_forum_topic_argument(method)?,
                        },
                    ],
                ]
            }
            ReviewedRuntimeContract::ConditionalPinRight => {
                let target = documented_chat_target(method)?;
                vec![
                    vec![documented_chat_kind(&target, ResolvedChatKind::Private)?],
                    vec![documented_chat_kind(&target, ResolvedChatKind::Secret)?],
                    vec![
                        documented_chat_kind(&target, ResolvedChatKind::BasicGroup)?,
                        RuntimeRequirement::ChatMemberRight {
                            target: target.clone(),
                            right: ChatMemberRight::CanPinMessages,
                        },
                    ],
                    vec![
                        documented_chat_kind(&target, ResolvedChatKind::Supergroup)?,
                        RuntimeRequirement::ChatMemberRight {
                            target: target.clone(),
                            right: ChatMemberRight::CanPinMessages,
                        },
                    ],
                    vec![
                        documented_chat_kind(&target, ResolvedChatKind::Channel)?,
                        RuntimeRequirement::ChatAdministratorRight {
                            target,
                            right: ChatAdministratorRight::CanEditMessages,
                        },
                    ],
                ]
            }
            ReviewedRuntimeContract::BusinessConnectionEnabledAndRight(right) => {
                let connection = documented_business_connection_argument(method)?;
                vec![vec![
                    RuntimeRequirement::BusinessConnectionEnabled {
                        connection: connection.clone(),
                    },
                    RuntimeRequirement::BusinessConnectionRight { connection, right },
                ]]
            }
        }
    };
    exact_runtime_alternatives(clauses).map(Some)
}

fn reviewed_supergroup_username_contract(
    method: &Definition,
) -> Result<
    Option<&'static supergroup_usernames::SupergroupUsernameContract>,
    CapabilityGenerationError,
> {
    let Some(contract) = supergroup_usernames::reviewed_contract(method.name()) else {
        return Ok(None);
    };
    if method.canonical_signature() != contract.canonical_signature() {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed supergroup-username signature drifted",
        ));
    }
    if !signal_source_has_exact_text(
        method,
        &RuntimeSignalSource::Description,
        contract.source_text(),
    ) {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed supergroup-username source text drifted or disappeared",
        ));
    }
    Ok(Some(contract))
}

fn reviewed_chat_invite_link_contract(
    method: &Definition,
) -> Result<Option<&'static chat_invite_links::ChatInviteLinkContract>, CapabilityGenerationError> {
    let Some(contract) = chat_invite_links::reviewed_contract(method.name()) else {
        return Ok(None);
    };
    if method.canonical_signature() != contract.canonical_signature() {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed chat-invite-link signature drifted",
        ));
    }
    if !signal_source_has_exact_text(
        method,
        &RuntimeSignalSource::Description,
        contract.source_text(),
    ) {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed chat-invite-link source text drifted or disappeared",
        ));
    }
    Ok(Some(contract))
}

fn reviewed_chat_event_log_contract(
    method: &Definition,
) -> Result<Option<&'static chat_event_logs::ChatEventLogContract>, CapabilityGenerationError> {
    let Some(contract) = chat_event_logs::reviewed_contract(method.name()) else {
        return Ok(None);
    };
    if method.canonical_signature() != contract.canonical_signature() {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed chat-event-log signature drifted",
        ));
    }
    if !signal_source_has_exact_text(
        method,
        &RuntimeSignalSource::Description,
        contract.source_text(),
    ) {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed chat-event-log source text drifted or disappeared",
        ));
    }
    Ok(Some(contract))
}

fn reviewed_chat_boost_contract(
    method: &Definition,
) -> Result<Option<&'static chat_boosts::ChatBoostContract>, CapabilityGenerationError> {
    let Some(contract) = chat_boosts::reviewed_contract(method.name()) else {
        return Ok(None);
    };
    if method.canonical_signature() != contract.canonical_signature() {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed chat-boost signature drifted",
        ));
    }
    if !signal_source_has_exact_text(
        method,
        &RuntimeSignalSource::Description,
        contract.source_text(),
    ) {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed chat-boost source text drifted or disappeared",
        ));
    }
    Ok(Some(contract))
}

fn reviewed_message_moderation_contract(
    method: &Definition,
) -> Result<Option<&'static message_moderation::MessageModerationContract>, CapabilityGenerationError>
{
    let Some(contract) = message_moderation::reviewed_contract(method.name()) else {
        return Ok(None);
    };
    if method.canonical_signature() != contract.canonical_signature() {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed message-moderation signature drifted",
        ));
    }
    if !signal_source_has_exact_text(
        method,
        &RuntimeSignalSource::Description,
        contract.source_text(),
    ) {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed message-moderation source text drifted or disappeared",
        ));
    }
    Ok(Some(contract))
}

fn reviewed_video_chat_contract(
    method: &Definition,
) -> Result<Option<&'static video_chats::VideoChatContract>, CapabilityGenerationError> {
    let Some(contract) = video_chats::reviewed_contract(method.name()) else {
        return Ok(None);
    };
    if method.canonical_signature() != contract.canonical_signature() {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed video-chat signature drifted",
        ));
    }
    if !signal_source_has_exact_text(
        method,
        &RuntimeSignalSource::Description,
        contract.source_text(),
    ) {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed video-chat source text drifted or disappeared",
        ));
    }
    Ok(Some(contract))
}

fn reviewed_chat_setting_contract(
    method: &Definition,
) -> Result<Option<&'static chat_settings::ChatSettingContract>, CapabilityGenerationError> {
    let Some(contract) = chat_settings::reviewed_contract(method.name()) else {
        return Ok(None);
    };
    if method.canonical_signature() != contract.canonical_signature() {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed chat-setting signature drifted",
        ));
    }
    if !signal_source_has_exact_text(
        method,
        &RuntimeSignalSource::Description,
        contract.source_text(),
    ) {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed chat-setting source text drifted or disappeared",
        ));
    }
    if let Some(source_text) = contract.target_source_text() {
        let target = documented_chat_target(method)?;
        if !signal_source_has_exact_text(
            method,
            &RuntimeSignalSource::Argument(target.argument().clone()),
            source_text,
        ) {
            return Err(unsupported_runtime_documentation(
                method,
                "reviewed chat-setting target source text drifted or disappeared",
            ));
        }
    }
    Ok(Some(contract))
}

fn chat_setting_consumed_signal_keys(
    contract: &chat_settings::ChatSettingContract,
) -> BTreeSet<RuntimeSignalKey> {
    let role_family = match contract.required_right() {
        chat_settings::RequiredRight::Administrator(_) => {
            RuntimeSignalFamily::AdministratorRightPhrase
        }
        chat_settings::RequiredRight::Member(_) => RuntimeSignalFamily::MemberRightPhrase,
    };
    [role_family, RuntimeSignalFamily::RequiresRightPhrase]
        .into_iter()
        .map(|family| RuntimeSignalKey {
            source: RuntimeSignalSource::Description,
            family,
        })
        .collect()
}

fn documented_chat_setting_clauses(
    method: &Definition,
    contract: &chat_settings::ChatSettingContract,
) -> Result<Vec<Vec<RuntimeRequirement>>, CapabilityGenerationError> {
    let target = documented_chat_target(method)?;
    contract
        .supported_chat_kinds()
        .iter()
        .copied()
        .map(|kind| {
            let mut clause = vec![documented_chat_kind(&target, kind)?];
            clause.extend(
                contract
                    .required_supergroup_flags()
                    .iter()
                    .map(|&(flag, value)| {
                        RuntimeRequirement::SupergroupFlag(SupergroupFlagCondition::new(
                            target.clone(),
                            flag,
                            value,
                        ))
                    }),
            );
            clause.push(match contract.required_right() {
                chat_settings::RequiredRight::Administrator(right) => {
                    RuntimeRequirement::ChatAdministratorRight {
                        target: target.clone(),
                        right,
                    }
                }
                chat_settings::RequiredRight::Member(right) => {
                    RuntimeRequirement::ChatMemberRight {
                        target: target.clone(),
                        right,
                    }
                }
            });
            Ok(clause)
        })
        .collect()
}

fn chat_invite_link_consumed_signal_keys(
    contract: &chat_invite_links::ChatInviteLinkContract,
) -> BTreeSet<RuntimeSignalKey> {
    let mut families = BTreeSet::new();
    match contract.required_access() {
        chat_invite_links::RequiredAccess::AdministratorRight(right) => {
            families.extend([
                RuntimeSignalFamily::RequiresAdministrator,
                RuntimeSignalFamily::RequiresRightPhrase,
                RuntimeSignalFamily::NamedRight(right),
            ]);
        }
        chat_invite_links::RequiredAccess::Owner => {
            families.insert(RuntimeSignalFamily::RequiresOwnerPrivileges);
        }
    }
    families
        .into_iter()
        .map(|family| RuntimeSignalKey {
            source: RuntimeSignalSource::Description,
            family,
        })
        .collect()
}

fn chat_administrator_consumed_signal_keys() -> BTreeSet<RuntimeSignalKey> {
    [RuntimeSignalKey {
        source: RuntimeSignalSource::Description,
        family: RuntimeSignalFamily::RequiresAdministrator,
    }]
    .into_iter()
    .collect()
}

fn message_moderation_consumed_signal_keys() -> BTreeSet<RuntimeSignalKey> {
    [
        RuntimeSignalFamily::AdministratorRightPhrase,
        RuntimeSignalFamily::RequiresRightPhrase,
    ]
    .into_iter()
    .map(|family| RuntimeSignalKey {
        source: RuntimeSignalSource::Description,
        family,
    })
    .collect()
}

fn documented_message_moderation_clauses(
    method: &Definition,
    contract: &message_moderation::MessageModerationContract,
) -> Result<Vec<Vec<RuntimeRequirement>>, CapabilityGenerationError> {
    let target = documented_chat_target(method)?;
    contract
        .supported_chat_kinds()
        .iter()
        .copied()
        .map(|kind| {
            let mut clause = vec![documented_chat_kind(&target, kind)?];
            if kind == ResolvedChatKind::Supergroup {
                clause.extend(
                    contract
                        .required_supergroup_flags()
                        .iter()
                        .map(|&(flag, value)| {
                            RuntimeRequirement::SupergroupFlag(SupergroupFlagCondition::new(
                                target.clone(),
                                flag,
                                value,
                            ))
                        }),
                );
            }
            clause.push(RuntimeRequirement::ChatAdministratorRight {
                target: target.clone(),
                right: contract.required_right(),
            });
            Ok(clause)
        })
        .collect()
}

fn video_chat_consumed_signal_keys(
    contract: &video_chats::VideoChatContract,
) -> BTreeSet<RuntimeSignalKey> {
    let families = match contract.required_access() {
        video_chats::RequiredAccess::AdministratorRight(_) => vec![
            RuntimeSignalFamily::AdministratorRightPhrase,
            RuntimeSignalFamily::RequiresRightPhrase,
        ],
        video_chats::RequiredAccess::Owner => {
            vec![RuntimeSignalFamily::RequiresOwnerPrivileges]
        }
    };
    families
        .into_iter()
        .map(|family| RuntimeSignalKey {
            source: RuntimeSignalSource::Description,
            family,
        })
        .collect()
}

fn supergroup_username_consumed_signal_keys() -> BTreeSet<RuntimeSignalKey> {
    [RuntimeSignalKey {
        source: RuntimeSignalSource::Description,
        family: RuntimeSignalFamily::RequiresOwnerPrivileges,
    }]
    .into_iter()
    .collect()
}

#[derive(Clone, Copy)]
enum ReviewedRuntimeContract {
    AdministratorInKinds(&'static [ResolvedChatKind]),
    AdministratorRightInKinds {
        right: ChatAdministratorRight,
        kinds: &'static [ResolvedChatKind],
    },
    OwnerInKind(ResolvedChatKind),
    AdministratorOrTopicCreator {
        right: ChatAdministratorRight,
        kind: ResolvedChatKind,
    },
    ConditionalPinRight,
    BusinessConnectionEnabledAndRight(BusinessBotRight),
}

impl ReviewedRuntimeContract {
    fn consumed_signal_keys(self) -> BTreeSet<RuntimeSignalKey> {
        let families: &[RuntimeSignalFamily] = match self {
            Self::AdministratorInKinds(_) => &[RuntimeSignalFamily::RequiresAdministrator],
            Self::AdministratorRightInKinds { .. } | Self::AdministratorOrTopicCreator { .. } => &[
                RuntimeSignalFamily::AdministratorRightPhrase,
                RuntimeSignalFamily::RequiresRightPhrase,
            ],
            Self::OwnerInKind(_) => &[RuntimeSignalFamily::RequiresOwnerPrivileges],
            Self::ConditionalPinRight => &[
                RuntimeSignalFamily::AdministratorRightPhrase,
                RuntimeSignalFamily::MemberRightPhrase,
                RuntimeSignalFamily::RequiresRightPhrase,
            ],
            Self::BusinessConnectionEnabledAndRight(_) => {
                &[RuntimeSignalFamily::RequiresRightPhrase]
            }
        };
        families
            .iter()
            .copied()
            .map(|family| RuntimeSignalKey {
                source: RuntimeSignalSource::Description,
                family,
            })
            .collect()
    }
}

fn reviewed_runtime_contract(method: &str, description: &str) -> Option<ReviewedRuntimeContract> {
    use ReviewedRuntimeContract as Contract;

    match (method, description) {
        (
            "upgradeBasicGroupChatToSupergroupChat",
            "creates a new supergroup from an existing basic group and sends a corresponding messagechatupgradeto and messagechatupgradefrom; requires owner privileges. deactivates the original basic group",
        ) => Some(Contract::OwnerInKind(ResolvedChatKind::BasicGroup)),
        (
            "setSupergroupStickerSet",
            "changes the sticker set of a supergroup; requires can_change_info administrator right",
        ) => Some(Contract::AdministratorRightInKinds {
            right: ChatAdministratorRight::CanChangeInfo,
            kinds: &[ResolvedChatKind::Supergroup],
        }),
        (
            "requireSyntheticSupergroupAdministrator",
            "requires administrator evidence in a synthetic supergroup fixture",
        ) => Some(Contract::AdministratorInKinds(&[
            ResolvedChatKind::Supergroup,
        ])),
        (
            "toggleForumTopicIsClosed",
            "toggles whether a topic is closed in a forum supergroup chat; requires can_manage_topics administrator right in the supergroup unless the user is creator of the topic",
        ) => Some(Contract::AdministratorOrTopicCreator {
            right: ChatAdministratorRight::CanManageTopics,
            kind: ResolvedChatKind::Supergroup,
        }),
        (
            "requireSyntheticConditionalPinRight",
            "removes a pinned message from a chat; requires can_pin_messages member right if the chat is a basic group or supergroup, or can_edit_messages administrator right if the chat is a channel",
        ) => Some(Contract::ConditionalPinRight),
        (
            "sendBusinessMessage",
            "sends on behalf of a business account; for bots only; requires an enabled business connection with can_reply right",
        ) => Some(Contract::BusinessConnectionEnabledAndRight(
            BusinessBotRight::CanReply,
        )),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum ReviewedMessageSignalSource {
    Description,
    Argument(&'static str),
}

impl ReviewedMessageSignalSource {
    fn runtime_source(self) -> Result<RuntimeSignalSource, CapabilityGenerationError> {
        match self {
            Self::Description => Ok(RuntimeSignalSource::Description),
            Self::Argument(argument) => ArgumentRef::try_from(argument)
                .map(RuntimeSignalSource::Argument)
                .map_err(|error| {
                    CapabilityGenerationError::new(
                        CapabilityGenerationErrorKind::SchemaDrift,
                        format!("reviewed message-property source isn't canonical: {error}"),
                    )
                }),
        }
    }
}

#[derive(Clone, Copy)]
enum ReviewedMessageSubject {
    One {
        chat_argument: &'static str,
        message_argument: &'static str,
    },
    Each {
        chat_argument: &'static str,
        message_argument: &'static str,
    },
}

#[derive(Clone, Copy)]
struct ReviewedMessageCapabilityContract {
    source: ReviewedMessageSignalSource,
    source_text: &'static str,
    subject: ReviewedMessageSubject,
    alternative_capabilities: &'static [MessageCapability],
    requires_supergroup_administrator: bool,
}

impl ReviewedMessageCapabilityContract {
    fn consumed_signal_keys(self) -> Result<BTreeSet<RuntimeSignalKey>, CapabilityGenerationError> {
        let source = self.source.runtime_source()?;
        let mut keys = [
            RuntimeSignalFamily::MessagePropertiesFact,
            RuntimeSignalFamily::CanFieldReference,
        ]
        .into_iter()
        .map(|family| RuntimeSignalKey {
            source: source.clone(),
            family,
        })
        .collect::<BTreeSet<_>>();
        if self.requires_supergroup_administrator {
            keys.insert(RuntimeSignalKey {
                source: RuntimeSignalSource::Description,
                family: RuntimeSignalFamily::RequiresAdministrator,
            });
        }
        Ok(keys)
    }
}

fn reviewed_message_capability_contract(
    method: &Definition,
) -> Result<Option<ReviewedMessageCapabilityContract>, CapabilityGenerationError> {
    use MessageCapability as Capability;
    use ReviewedMessageSignalSource as Source;

    const ONE: ReviewedMessageSubject = ReviewedMessageSubject::One {
        chat_argument: "chat_id",
        message_argument: "message_id",
    };
    const EACH_SPAM_MESSAGE: ReviewedMessageSubject = ReviewedMessageSubject::Each {
        chat_argument: "supergroup_id",
        message_argument: "message_ids",
    };

    let Some(contract) = (match method.name() {
        "addChecklistTasks" => Some(ReviewedMessageCapabilityContract {
            source: Source::Argument("message_id"),
            source_text: "identifier of the message containing the checklist. use messageproperties.can_add_tasks to check whether the tasks can be added",
            subject: ONE,
            alternative_capabilities: &[Capability::CanAddTasks],
            requires_supergroup_administrator: false,
        }),
        "addOffer" => Some(ReviewedMessageCapabilityContract {
            source: Source::Argument("message_id"),
            source_text: "identifier of the message in the chat which will be sent as suggested post. use messageproperties.can_add_offer to check whether an offer can be added or messageproperties.can_edit_suggested_post_info to check whether price or time of sending of the post can be changed",
            subject: ONE,
            alternative_capabilities: &[
                Capability::CanAddOffer,
                Capability::CanEditSuggestedPostInfo,
            ],
            requires_supergroup_administrator: false,
        }),
        "approveSuggestedPost" => Some(ReviewedMessageCapabilityContract {
            source: Source::Argument("message_id"),
            source_text: "identifier of the message with the suggested post. use messageproperties.can_be_approved to check whether the suggested post can be approved",
            subject: ONE,
            alternative_capabilities: &[Capability::CanBeApproved],
            requires_supergroup_administrator: false,
        }),
        "declineSuggestedPost" => Some(ReviewedMessageCapabilityContract {
            source: Source::Argument("message_id"),
            source_text: "identifier of the message with the suggested post. use messageproperties.can_be_declined to check whether the suggested post can be declined",
            subject: ONE,
            alternative_capabilities: &[Capability::CanBeDeclined],
            requires_supergroup_administrator: false,
        }),
        "deleteMessageReactionsFromSender" => Some(ReviewedMessageCapabilityContract {
            source: Source::Argument("message_id"),
            source_text: "identifier of the message containing the reactions. use messageproperties.can_delete_reactions to check whether the method can be used for a message",
            subject: ONE,
            alternative_capabilities: &[Capability::CanDeleteReactions],
            requires_supergroup_administrator: false,
        }),
        "editMessageCaption"
        | "editMessageChecklist"
        | "editMessageLiveLocation"
        | "editMessageReplyMarkup"
        | "editMessageText" => Some(ReviewedMessageCapabilityContract {
            source: Source::Argument("message_id"),
            source_text: "identifier of the message. use messageproperties.can_be_edited to check whether the message can be edited",
            subject: ONE,
            alternative_capabilities: &[Capability::CanBeEdited],
            requires_supergroup_administrator: false,
        }),
        "editMessageMedia" => Some(ReviewedMessageCapabilityContract {
            source: Source::Argument("message_id"),
            source_text: "identifier of the message. use messageproperties.can_edit_media to check whether the message can be edited",
            subject: ONE,
            alternative_capabilities: &[Capability::CanEditMedia],
            requires_supergroup_administrator: false,
        }),
        "editMessageSchedulingState" => Some(ReviewedMessageCapabilityContract {
            source: Source::Argument("message_id"),
            source_text: "identifier of the message. use messageproperties.can_edit_scheduling_state to check whether the message is suitable",
            subject: ONE,
            alternative_capabilities: &[Capability::CanEditSchedulingState],
            requires_supergroup_administrator: false,
        }),
        "getMessageAuthor" => Some(ReviewedMessageCapabilityContract {
            source: Source::Description,
            source_text: "returns information about actual author of a message sent on behalf of a channel. the method can be called if messageproperties.can_get_author == true",
            subject: ONE,
            alternative_capabilities: &[Capability::CanGetAuthor],
            requires_supergroup_administrator: false,
        }),
        "getMessageEmbeddingCode" => Some(ReviewedMessageCapabilityContract {
            source: Source::Description,
            source_text: "returns an html code for embedding the message. available only if messageproperties.can_get_embedding_code",
            subject: ONE,
            alternative_capabilities: &[Capability::CanGetEmbeddingCode],
            requires_supergroup_administrator: false,
        }),
        "getMessagePublicForwards" => Some(ReviewedMessageCapabilityContract {
            source: Source::Description,
            source_text: "returns forwarded copies of a channel message to different public channels and public reposts as a story. can be used only if messageproperties.can_get_statistics == true. for optimal performance, the number of returned messages and stories is chosen by tdlib",
            subject: ONE,
            alternative_capabilities: &[Capability::CanGetStatistics],
            requires_supergroup_administrator: false,
        }),
        "getMessageReadDate" => Some(ReviewedMessageCapabilityContract {
            source: Source::Description,
            source_text: "returns read date of a recent outgoing message in a private chat. the method can be called if messageproperties.can_get_read_date == true",
            subject: ONE,
            alternative_capabilities: &[Capability::CanGetReadDate],
            requires_supergroup_administrator: false,
        }),
        "getMessageStatistics" => Some(ReviewedMessageCapabilityContract {
            source: Source::Description,
            source_text: "returns detailed statistics about a message. can be used only if messageproperties.can_get_statistics == true",
            subject: ONE,
            alternative_capabilities: &[Capability::CanGetStatistics],
            requires_supergroup_administrator: false,
        }),
        "getMessageThread" => Some(ReviewedMessageCapabilityContract {
            source: Source::Description,
            source_text: "returns information about a message thread. can be used only if messageproperties.can_get_message_thread == true",
            subject: ONE,
            alternative_capabilities: &[Capability::CanGetMessageThread],
            requires_supergroup_administrator: false,
        }),
        "getMessageThreadHistory" => Some(ReviewedMessageCapabilityContract {
            source: Source::Description,
            source_text: "returns messages in a message thread of a message. can be used only if messageproperties.can_get_message_thread == true. message thread of a channel message is in the channel's linked supergroup. the messages are returned in reverse chronological order (i.e., in order of decreasing message_id). for optimal performance, the number of returned messages is chosen by tdlib",
            subject: ONE,
            alternative_capabilities: &[Capability::CanGetMessageThread],
            requires_supergroup_administrator: false,
        }),
        "getMessageViewers" => Some(ReviewedMessageCapabilityContract {
            source: Source::Description,
            source_text: "returns viewers of a recent outgoing message in a basic group or a supergroup chat. for video notes and voice notes only users, opened content of the message, are returned. the method can be called if messageproperties.can_get_viewers == true",
            subject: ONE,
            alternative_capabilities: &[Capability::CanGetViewers],
            requires_supergroup_administrator: false,
        }),
        "getPollVoteStatistics" => Some(ReviewedMessageCapabilityContract {
            source: Source::Argument("message_id"),
            source_text: "identifier of the message containing the poll. use messageproperties.can_get_poll_vote_statistics to check whether the method can be used for a message",
            subject: ONE,
            alternative_capabilities: &[Capability::CanGetPollVoteStatistics],
            requires_supergroup_administrator: false,
        }),
        "getVideoMessageAdvertisements" => Some(ReviewedMessageCapabilityContract {
            source: Source::Description,
            source_text: "returns advertisements to be shown while a video from a message is watched. available only if messageproperties.can_get_video_advertisements",
            subject: ONE,
            alternative_capabilities: &[Capability::CanGetVideoAdvertisements],
            requires_supergroup_administrator: false,
        }),
        "markChecklistTasksAsDone" => Some(ReviewedMessageCapabilityContract {
            source: Source::Argument("message_id"),
            source_text: "identifier of the message containing the checklist. use messageproperties.can_mark_tasks_as_done to check whether the tasks can be marked as done or not done",
            subject: ONE,
            alternative_capabilities: &[Capability::CanMarkTasksAsDone],
            requires_supergroup_administrator: false,
        }),
        "pinChatMessage" => Some(ReviewedMessageCapabilityContract {
            source: Source::Description,
            source_text: "pins a message in a chat. a message can be pinned only if messageproperties.can_be_pinned",
            subject: ONE,
            alternative_capabilities: &[Capability::CanBePinned],
            requires_supergroup_administrator: false,
        }),
        "recognizeSpeech" => Some(ReviewedMessageCapabilityContract {
            source: Source::Argument("message_id"),
            source_text: "identifier of the message. use messageproperties.can_recognize_speech to check whether the message is suitable",
            subject: ONE,
            alternative_capabilities: &[Capability::CanRecognizeSpeech],
            requires_supergroup_administrator: false,
        }),
        "reportMessageReactions" => Some(ReviewedMessageCapabilityContract {
            source: Source::Description,
            source_text: "reports reactions set on a message to the telegram moderators. reactions on a message can be reported only if messageproperties.can_report_reactions",
            subject: ONE,
            alternative_capabilities: &[Capability::CanReportReactions],
            requires_supergroup_administrator: false,
        }),
        "reportSupergroupSpam" => Some(ReviewedMessageCapabilityContract {
            source: Source::Argument("message_ids"),
            source_text: "identifiers of messages to report. use messageproperties.can_report_supergroup_spam to check whether the message can be reported",
            subject: EACH_SPAM_MESSAGE,
            alternative_capabilities: &[Capability::CanReportSupergroupSpam],
            requires_supergroup_administrator: true,
        }),
        "setMessageFactCheck" => Some(ReviewedMessageCapabilityContract {
            source: Source::Description,
            source_text: "changes the fact-check of a message. can be only used if messageproperties.can_set_fact_check == true",
            subject: ONE,
            alternative_capabilities: &[Capability::CanSetFactCheck],
            requires_supergroup_administrator: false,
        }),
        "stopPoll" => Some(ReviewedMessageCapabilityContract {
            source: Source::Argument("message_id"),
            source_text: "identifier of the message containing the poll. use messageproperties.can_be_edited to check whether the poll can be stopped",
            subject: ONE,
            alternative_capabilities: &[Capability::CanBeEdited],
            requires_supergroup_administrator: false,
        }),
        _ => None,
    }) else {
        return Ok(None);
    };

    let source = contract.source.runtime_source()?;
    if !signal_source_has_exact_text(method, &source, contract.source_text) {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed message-property source text drifted or disappeared",
        ));
    }
    if contract.requires_supergroup_administrator
        && !signal_source_has_exact_text(
            method,
            &RuntimeSignalSource::Description,
            "reports messages in a supergroup as spam; requires administrator rights in the supergroup",
        )
    {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed supergroup-spam administrator source text drifted or disappeared",
        ));
    }
    Ok(Some(contract))
}

#[derive(Clone, Copy)]
enum ReviewedGroupCallFormula {
    Property(GroupCallProperty),
    KindAndProperty {
        kind: ResolvedGroupCallKind,
        property: GroupCallProperty,
    },
    KindAndMessages {
        kind: ResolvedGroupCallKind,
        capability: GroupCallMessageCapability,
    },
    VideoManagedOrUnboundOwned,
    ManagedBoundKindOrUnboundOwned,
}

#[derive(Clone, Copy)]
struct ReviewedGroupCallContract {
    source_text: &'static str,
    consumed_families: &'static [RuntimeSignalFamily],
    formula: ReviewedGroupCallFormula,
}

impl ReviewedGroupCallContract {
    fn consumed_signal_keys(self) -> BTreeSet<RuntimeSignalKey> {
        self.consumed_families
            .iter()
            .copied()
            .map(|family| RuntimeSignalKey {
                source: RuntimeSignalSource::Description,
                family,
            })
            .collect()
    }
}

fn reviewed_group_call_contract(
    method: &Definition,
) -> Result<Option<ReviewedGroupCallContract>, CapabilityGenerationError> {
    use GroupCallMessageCapability as MessageCapability;
    use GroupCallProperty as Property;
    use ResolvedGroupCallKind as Kind;
    use ReviewedGroupCallFormula as Formula;

    const PROPERTY_FACT: &[RuntimeSignalFamily] = &[
        RuntimeSignalFamily::RequiresRightPhrase,
        RuntimeSignalFamily::GroupCallFact,
        RuntimeSignalFamily::CanFieldReference,
    ];
    const MESSAGE_PROPERTY_FACT: &[RuntimeSignalFamily] = &[
        RuntimeSignalFamily::RequiresRightPhrase,
        RuntimeSignalFamily::GroupCallMessageFact,
        RuntimeSignalFamily::CanFieldReference,
    ];
    const DELETE_PROPERTY_FACT: &[RuntimeSignalFamily] = &[
        RuntimeSignalFamily::RequiresRightPhrase,
        RuntimeSignalFamily::GroupCallFact,
        RuntimeSignalFamily::CanFieldReference,
        RuntimeSignalFamily::NamedRight(ChatAdministratorRight::CanDeleteMessages),
    ];
    const OWNERSHIP_FACT: &[RuntimeSignalFamily] = &[
        RuntimeSignalFamily::GroupCallFact,
        RuntimeSignalFamily::IsFieldReference,
    ];
    const MANAGED_OR_OWNED_FACT: &[RuntimeSignalFamily] = &[
        RuntimeSignalFamily::RequiresRightPhrase,
        RuntimeSignalFamily::GroupCallFact,
        RuntimeSignalFamily::CanFieldReference,
        RuntimeSignalFamily::IsFieldReference,
    ];

    let Some(contract) = (match method.name() {
        "setVideoChatTitle" => Some(ReviewedGroupCallContract {
            source_text: "sets title of a video chat; requires groupcall.can_be_managed right",
            consumed_families: PROPERTY_FACT,
            formula: Formula::KindAndProperty {
                kind: Kind::VideoChat,
                property: Property::CanBeManaged,
            },
        }),
        "toggleVideoChatMuteNewParticipants" => Some(ReviewedGroupCallContract {
            source_text: "toggles whether new participants of a video chat can be unmuted only by administrators of the video chat. requires groupcall.can_toggle_mute_new_participants right",
            consumed_families: PROPERTY_FACT,
            formula: Formula::KindAndProperty {
                kind: Kind::VideoChat,
                property: Property::CanToggleMuteNewParticipants,
            },
        }),
        "toggleGroupCallAreMessagesAllowed" => Some(ReviewedGroupCallContract {
            source_text: "toggles whether participants of a group call can send messages there. requires groupcall.can_toggle_are_messages_allowed right",
            consumed_families: PROPERTY_FACT,
            formula: Formula::Property(Property::CanToggleAreMessagesAllowed),
        }),
        "sendGroupCallMessage" => Some(ReviewedGroupCallContract {
            source_text: "sends a message to other participants of a group call. requires groupcall.can_send_messages right",
            consumed_families: PROPERTY_FACT,
            formula: Formula::Property(Property::CanSendMessages),
        }),
        "deleteGroupCallMessages" => Some(ReviewedGroupCallContract {
            source_text: "deletes messages in a group call; for live story calls only. requires groupcallmessage.can_be_deleted right",
            consumed_families: MESSAGE_PROPERTY_FACT,
            formula: Formula::KindAndMessages {
                kind: Kind::LiveStory,
                capability: MessageCapability::CanBeDeleted,
            },
        }),
        "deleteGroupCallMessagesBySender" => Some(ReviewedGroupCallContract {
            source_text: "deletes all messages sent by the specified message sender in a group call; for live story calls only. requires groupcall.can_delete_messages right",
            consumed_families: DELETE_PROPERTY_FACT,
            formula: Formula::KindAndProperty {
                kind: Kind::LiveStory,
                property: Property::CanDeleteMessages,
            },
        }),
        "banGroupCallParticipants" => Some(ReviewedGroupCallContract {
            source_text: "bans users from a group call not bound to a chat; requires groupcall.is_owned. only the owner of the group call can invite the banned users back",
            consumed_families: OWNERSHIP_FACT,
            formula: Formula::KindAndProperty {
                kind: Kind::Unbound,
                property: Property::IsOwned,
            },
        }),
        "revokeGroupCallInviteLink" => Some(ReviewedGroupCallContract {
            source_text: "revokes invite link for a group call. requires groupcall.can_be_managed right for video chats or groupcall.is_owned otherwise",
            consumed_families: MANAGED_OR_OWNED_FACT,
            formula: Formula::VideoManagedOrUnboundOwned,
        }),
        "startGroupCallRecording" => Some(ReviewedGroupCallContract {
            source_text: "starts recording of an active group call; for video chats only. requires groupcall.can_be_managed right",
            consumed_families: PROPERTY_FACT,
            formula: Formula::KindAndProperty {
                kind: Kind::VideoChat,
                property: Property::CanBeManaged,
            },
        }),
        "endGroupCallRecording" => Some(ReviewedGroupCallContract {
            source_text: "ends recording of an active group call; for video chats only. requires groupcall.can_be_managed right",
            consumed_families: PROPERTY_FACT,
            formula: Formula::KindAndProperty {
                kind: Kind::VideoChat,
                property: Property::CanBeManaged,
            },
        }),
        "setGroupCallPaidMessageStarCount" => Some(ReviewedGroupCallContract {
            source_text: "changes the minimum number of telegram stars that must be paid by general participant for each sent message to a live story call. requires groupcall.can_be_managed right",
            consumed_families: PROPERTY_FACT,
            formula: Formula::KindAndProperty {
                kind: Kind::LiveStory,
                property: Property::CanBeManaged,
            },
        }),
        "endGroupCall" => Some(ReviewedGroupCallContract {
            source_text: "ends a group call. requires groupcall.can_be_managed right for video chats and live stories or groupcall.is_owned otherwise",
            consumed_families: MANAGED_OR_OWNED_FACT,
            formula: Formula::ManagedBoundKindOrUnboundOwned,
        }),
        _ => None,
    }) else {
        return Ok(None);
    };

    if !signal_source_has_exact_text(
        method,
        &RuntimeSignalSource::Description,
        contract.source_text,
    ) {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed group-call source text drifted or disappeared",
        ));
    }
    Ok(Some(contract))
}

fn documented_group_call_clauses(
    method: &Definition,
    contract: ReviewedGroupCallContract,
) -> Result<Vec<Vec<RuntimeRequirement>>, CapabilityGenerationError> {
    use GroupCallProperty as Property;
    use ResolvedGroupCallKind as Kind;
    use ReviewedGroupCallFormula as Formula;

    let group_call = documented_group_call_id(method)?;
    let kind = |kind| {
        RuntimeRequirement::GroupCallKind(GroupCallKindCondition::new(group_call.clone(), kind))
    };
    let property = |property| RuntimeRequirement::GroupCallProperty {
        group_call: group_call.clone(),
        property,
    };
    Ok(match contract.formula {
        Formula::Property(value) => vec![vec![property(value)]],
        Formula::KindAndProperty {
            kind: value,
            property: required,
        } => vec![vec![kind(value), property(required)]],
        Formula::KindAndMessages {
            kind: value,
            capability,
        } => vec![vec![
            kind(value),
            RuntimeRequirement::GroupCallMessageCapability {
                subject: documented_group_call_message_subject(method, group_call.clone())?,
                capability,
            },
        ]],
        Formula::VideoManagedOrUnboundOwned => vec![
            vec![kind(Kind::VideoChat), property(Property::CanBeManaged)],
            vec![kind(Kind::Unbound), property(Property::IsOwned)],
        ],
        Formula::ManagedBoundKindOrUnboundOwned => vec![
            vec![kind(Kind::VideoChat), property(Property::CanBeManaged)],
            vec![kind(Kind::LiveStory), property(Property::CanBeManaged)],
            vec![kind(Kind::Unbound), property(Property::IsOwned)],
        ],
    })
}

#[derive(Clone, Copy)]
enum ReviewedSupergroupFullInfoFormula {
    Property(SupergroupFullInfoProperty),
    AdministratorRightAndProperty {
        right: ChatAdministratorRight,
        property: SupergroupFullInfoProperty,
    },
}

#[derive(Clone, Copy)]
struct ReviewedSupergroupFullInfoContract {
    source_text: &'static str,
    consumed_families: &'static [RuntimeSignalFamily],
    formula: ReviewedSupergroupFullInfoFormula,
}

const TOGGLE_SUPERGROUP_HAS_HIDDEN_MEMBERS_DESCRIPTION: &str = "toggles whether non-administrators can receive only administrators and bots using getsupergroupmembers or searchchatmembers. can be called only if supergroupfullinfo.can_hide_members == true";
const GET_SUPERGROUP_MEMBERS_DESCRIPTION: &str = "returns information about members or banned users in a supergroup or channel. can be used only if supergroupfullinfo.can_get_members == true; additionally, administrator privileges may be required for some filters";

impl ReviewedSupergroupFullInfoContract {
    fn consumed_signal_keys(self) -> BTreeSet<RuntimeSignalKey> {
        self.consumed_families
            .iter()
            .copied()
            .map(|family| RuntimeSignalKey {
                source: RuntimeSignalSource::Description,
                family,
            })
            .collect()
    }
}

fn reviewed_supergroup_full_info_contract(
    method: &Definition,
) -> Result<Option<ReviewedSupergroupFullInfoContract>, CapabilityGenerationError> {
    use ReviewedSupergroupFullInfoFormula as Formula;
    use SupergroupFullInfoProperty as Property;

    const PROPERTY_FACT: &[RuntimeSignalFamily] = &[
        RuntimeSignalFamily::SupergroupFullInfoFact,
        RuntimeSignalFamily::CanFieldReference,
    ];
    const ADMINISTRATOR_PROPERTY_FACT: &[RuntimeSignalFamily] = &[
        RuntimeSignalFamily::AdministratorRightPhrase,
        RuntimeSignalFamily::RequiresRightPhrase,
        RuntimeSignalFamily::SupergroupFullInfoFact,
        RuntimeSignalFamily::CanFieldReference,
    ];

    let Some(contract) = (match method.name() {
        "getChatStatistics" => Some(ReviewedSupergroupFullInfoContract {
            source_text: "returns detailed statistics about a chat. currently, this method can be used only for supergroups and channels. can be used only if supergroupfullinfo.can_get_statistics == true",
            consumed_families: PROPERTY_FACT,
            formula: Formula::Property(Property::CanGetStatistics),
        }),
        "setChatLocation" => Some(ReviewedSupergroupFullInfoContract {
            source_text: "changes the location of a chat. available only for some location-based supergroups, use supergroupfullinfo.can_set_location to check whether the method is allowed to use",
            consumed_families: PROPERTY_FACT,
            formula: Formula::Property(Property::CanSetLocation),
        }),
        "setChatPaidMessageStarCount" => Some(ReviewedSupergroupFullInfoContract {
            source_text: "changes the telegram star amount that must be paid to send a message to a supergroup chat; requires can_restrict_members administrator right and supergroupfullinfo.can_enable_paid_messages",
            consumed_families: ADMINISTRATOR_PROPERTY_FACT,
            formula: Formula::AdministratorRightAndProperty {
                right: ChatAdministratorRight::CanRestrictMembers,
                property: Property::CanEnablePaidMessages,
            },
        }),
        "toggleSupergroupHasAggressiveAntiSpamEnabled" => {
            Some(ReviewedSupergroupFullInfoContract {
                source_text: "toggles whether aggressive anti-spam checks are enabled in the supergroup. can be called only if supergroupfullinfo.can_toggle_aggressive_anti_spam == true",
                consumed_families: PROPERTY_FACT,
                formula: Formula::Property(Property::CanToggleAggressiveAntiSpam),
            })
        }
        "toggleSupergroupHasHiddenMembers" => Some(ReviewedSupergroupFullInfoContract {
            source_text: TOGGLE_SUPERGROUP_HAS_HIDDEN_MEMBERS_DESCRIPTION,
            consumed_families: PROPERTY_FACT,
            formula: Formula::Property(Property::CanHideMembers),
        }),
        _ => None,
    }) else {
        return Ok(None);
    };

    if !signal_source_has_exact_text(
        method,
        &RuntimeSignalSource::Description,
        contract.source_text,
    ) {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed supergroup-full-info source text drifted or disappeared",
        ));
    }
    Ok(Some(contract))
}

fn documented_supergroup_full_info_clauses(
    method: &Definition,
    contract: ReviewedSupergroupFullInfoContract,
) -> Result<Vec<Vec<RuntimeRequirement>>, CapabilityGenerationError> {
    use ReviewedSupergroupFullInfoFormula as Formula;

    let target = documented_chat_target(method)?;
    let property = |property| RuntimeRequirement::SupergroupFullInfoProperty {
        target: target.clone(),
        property,
    };
    Ok(match contract.formula {
        Formula::Property(value) => vec![vec![property(value)]],
        Formula::AdministratorRightAndProperty {
            right,
            property: required,
        } => vec![vec![
            RuntimeRequirement::ChatAdministratorRight {
                target: target.clone(),
                right,
            },
            property(required),
        ]],
    })
}

#[derive(Clone, Copy)]
struct ReviewedRuntimeBooleanOptionContract {
    source_text: &'static str,
    canonical_signature: &'static str,
    option: RuntimeBooleanOption,
    regular_user_only: bool,
}

impl ReviewedRuntimeBooleanOptionContract {
    fn consumed_signal_keys(self) -> BTreeSet<RuntimeSignalKey> {
        [RuntimeSignalKey {
            source: RuntimeSignalSource::Description,
            family: RuntimeSignalFamily::OptionGate,
        }]
        .into_iter()
        .collect()
    }
}

fn reviewed_runtime_boolean_option_contract(
    method: &Definition,
) -> Result<Option<ReviewedRuntimeBooleanOptionContract>, CapabilityGenerationError> {
    let Some(contract) = (match method.name() {
        "setNewChatPrivacySettings" => Some(ReviewedRuntimeBooleanOptionContract {
            source_text: "changes privacy settings for new chat creation; can be used only if getoption(\"can_set_new_chat_privacy_settings\")",
            canonical_signature: "setNewChatPrivacySettings settings:newChatPrivacySettings = Ok;",
            option: RuntimeBooleanOption::CanSetNewChatPrivacySettings,
            regular_user_only: true,
        }),
        _ => None,
    }) else {
        return Ok(None);
    };

    if method.canonical_signature() != contract.canonical_signature {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed boolean-option method signature drifted",
        ));
    }
    if !signal_source_has_exact_text(
        method,
        &RuntimeSignalSource::Description,
        contract.source_text,
    ) {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed boolean-option source text drifted or disappeared",
        ));
    }
    Ok(Some(contract))
}

fn documented_group_call_id(
    method: &Definition,
) -> Result<GroupCallIdRef, CapabilityGenerationError> {
    let Some(ty) = field_type(method, "group_call_id") else {
        return Err(unsupported_runtime_documentation(
            method,
            "group-call contract is missing group_call_id",
        ));
    };
    if ty.name() != "int32" || !ty.arguments().is_empty() {
        return Err(unsupported_runtime_documentation(
            method,
            "group_call_id must have exact int32 type",
        ));
    }
    GroupCallIdRef::try_from("group_call_id").map_err(|error| {
        CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            error.to_string(),
        )
    })
}

fn documented_group_call_message_subject(
    method: &Definition,
    group_call: GroupCallIdRef,
) -> Result<GroupCallMessageSubjectRef, CapabilityGenerationError> {
    let Some(ty) = field_type(method, "message_ids") else {
        return Err(unsupported_runtime_documentation(
            method,
            "group-call message contract is missing message_ids",
        ));
    };
    let exact = ty.name() == "vector"
        && matches!(ty.arguments(), [element] if element.name() == "int32" && element.arguments().is_empty());
    if !exact {
        return Err(unsupported_runtime_documentation(
            method,
            "universal group-call message contract requires exact vector<int32>",
        ));
    }
    Ok(GroupCallMessageSubjectRef::Each {
        group_call,
        messages: GroupCallMessageIdsRef::try_from("message_ids").map_err(|error| {
            CapabilityGenerationError::new(
                CapabilityGenerationErrorKind::SchemaDrift,
                error.to_string(),
            )
        })?,
    })
}

fn documented_message_capability_clauses(
    method: &Definition,
    contract: ReviewedMessageCapabilityContract,
) -> Result<Vec<Vec<RuntimeRequirement>>, CapabilityGenerationError> {
    let subject = documented_message_subject(method, contract.subject)?;
    let message_requirements = contract
        .alternative_capabilities
        .iter()
        .copied()
        .map(|capability| RuntimeRequirement::MessageCapability {
            subject: subject.clone(),
            capability,
        })
        .collect::<Vec<_>>();
    if contract.requires_supergroup_administrator {
        let [message_requirement] = message_requirements.as_slice() else {
            return Err(unsupported_runtime_documentation(
                method,
                "administrator-scoped message contract needs exactly one message predicate",
            ));
        };
        let target = subject.chat().clone();
        return Ok(vec![vec![
            documented_chat_kind(&target, ResolvedChatKind::Supergroup)?,
            RuntimeRequirement::ChatAdministrator {
                target: target.clone(),
            },
            message_requirement.clone(),
        ]]);
    }
    Ok(message_requirements
        .into_iter()
        .map(|requirement| vec![requirement])
        .collect())
}

fn documented_message_subject(
    method: &Definition,
    subject: ReviewedMessageSubject,
) -> Result<MessageSubjectRef, CapabilityGenerationError> {
    let target = documented_chat_target(method)?;
    let (chat_argument, message_argument, each) = match subject {
        ReviewedMessageSubject::One {
            chat_argument,
            message_argument,
        } => (chat_argument, message_argument, false),
        ReviewedMessageSubject::Each {
            chat_argument,
            message_argument,
        } => (chat_argument, message_argument, true),
    };
    if target.argument().as_str() != chat_argument {
        return Err(unsupported_runtime_documentation(
            method,
            "message-property contract uses a different chat identifier space",
        ));
    }
    let Some(ty) = field_type(method, message_argument) else {
        return Err(unsupported_runtime_documentation(
            method,
            "message-property contract is missing its semantic message argument",
        ));
    };
    if each {
        let exact = ty.name() == "vector"
            && matches!(ty.arguments(), [element] if element.name() == "int53" && element.arguments().is_empty());
        if !exact {
            return Err(unsupported_runtime_documentation(
                method,
                "universal message-property contract requires exact vector<int53>",
            ));
        }
        let messages = MessageIdsRef::try_from(message_argument).map_err(|error| {
            CapabilityGenerationError::new(
                CapabilityGenerationErrorKind::SchemaDrift,
                error.to_string(),
            )
        })?;
        Ok(MessageSubjectRef::Each {
            chat: target,
            messages,
        })
    } else {
        if ty.name() != "int53" || !ty.arguments().is_empty() {
            return Err(unsupported_runtime_documentation(
                method,
                "scalar message-property contract requires exact int53",
            ));
        }
        let message = MessageIdRef::try_from(message_argument).map_err(|error| {
            CapabilityGenerationError::new(
                CapabilityGenerationErrorKind::SchemaDrift,
                error.to_string(),
            )
        })?;
        Ok(MessageSubjectRef::One {
            chat: target,
            message,
        })
    }
}

fn documented_chat_kind(
    target: &ChatTargetRef,
    kind: ResolvedChatKind,
) -> Result<RuntimeRequirement, CapabilityGenerationError> {
    ChatKindCondition::try_new(target.clone(), kind)
        .map(RuntimeRequirement::ChatKind)
        .map_err(|error| {
            CapabilityGenerationError::new(
                CapabilityGenerationErrorKind::SchemaDrift,
                format!("documented chat-kind gate isn't canonical: {error}"),
            )
        })
}

fn chat_kind_clauses(
    target: &ChatTargetRef,
    kinds: &[ResolvedChatKind],
    trailing: impl Fn(&ChatTargetRef) -> RuntimeRequirement,
) -> Result<Vec<Vec<RuntimeRequirement>>, CapabilityGenerationError> {
    kinds
        .iter()
        .copied()
        .map(|kind| Ok(vec![documented_chat_kind(target, kind)?, trailing(target)]))
        .collect()
}

fn exact_runtime_alternatives(
    clauses: Vec<Vec<RuntimeRequirement>>,
) -> Result<RequirementAlternatives, CapabilityGenerationError> {
    RequirementAlternatives::try_new(clauses).map_err(|error| {
        CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            format!("documented runtime gate isn't canonical: {error}"),
        )
    })
}

fn documented_chat_target(method: &Definition) -> Result<ChatTargetRef, CapabilityGenerationError> {
    let mut targets = Vec::new();
    for name in ["chat_id", "supergroup_id"] {
        if let Some(ty) = field_type(method, name) {
            if ty.name() != "int53" || !ty.arguments().is_empty() {
                return Err(unsupported_runtime_documentation(
                    method,
                    "semantic chat target has a non-int53 schema type",
                ));
            }
            let target = ChatTargetRef::try_from(name).map_err(|error| {
                CapabilityGenerationError::new(
                    CapabilityGenerationErrorKind::SchemaDrift,
                    error.to_string(),
                )
            })?;
            targets.push(target);
        }
    }
    match targets.as_slice() {
        [target] => Ok(target.clone()),
        _ => Err(unsupported_runtime_documentation(
            method,
            "runtime gate needs exactly one semantic chat_id or supergroup_id target",
        )),
    }
}

fn documented_forum_topic_argument(
    method: &Definition,
) -> Result<ForumTopicRef, CapabilityGenerationError> {
    let Some(ty) = field_type(method, "forum_topic_id") else {
        return Err(unsupported_runtime_documentation(
            method,
            "topic-creator gate needs forum_topic_id",
        ));
    };
    if ty.name() != "int32" || !ty.arguments().is_empty() {
        return Err(unsupported_runtime_documentation(
            method,
            "forum_topic_id must have exact int32 type",
        ));
    }
    ForumTopicRef::try_from("forum_topic_id").map_err(|error| {
        CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            error.to_string(),
        )
    })
}

fn documented_business_connection_argument(
    method: &Definition,
) -> Result<BusinessConnectionRef, CapabilityGenerationError> {
    let name = "business_connection_id";
    let Some(ty) = field_type(method, name) else {
        return Err(unsupported_runtime_documentation(
            method,
            "runtime gate needs a semantic business_connection_id",
        ));
    };
    if ty.name() != "string" || !ty.arguments().is_empty() {
        return Err(unsupported_runtime_documentation(
            method,
            "semantic business connection has a non-string schema type",
        ));
    }
    BusinessConnectionRef::try_from(name).map_err(|error| {
        CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            error.to_string(),
        )
    })
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum RuntimeSignalSource {
    Description,
    Argument(ArgumentRef),
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum RuntimeSignalFamily {
    AdministratorRightPhrase,
    MemberRightPhrase,
    RequiresAdministrator,
    RequiresOwnerPrivileges,
    RequiresRightPhrase,
    GroupCallFact,
    GroupCallMessageFact,
    SupergroupFullInfoFact,
    MessagePropertiesFact,
    ChatBoostReference,
    BoostLevelPhrase,
    CanFieldReference,
    IsFieldReference,
    UserIsAdministrator,
    MustBeAdministrator,
    MustHaveAdministratorPrivileges,
    NamedRight(ChatAdministratorRight),
    OnlyByAdministrator,
    OnlyIfAdministrator,
    AdministratorPrivilegesMayBeRequired,
    AdministratorRightsMayBeRequired,
    OptionGate,
}

#[cfg(test)]
impl RuntimeSignalFamily {
    fn canonical_name(self) -> String {
        match self {
            Self::AdministratorRightPhrase => "administrator_right_phrase".to_owned(),
            Self::MemberRightPhrase => "member_right_phrase".to_owned(),
            Self::RequiresAdministrator => "requires_administrator".to_owned(),
            Self::RequiresOwnerPrivileges => "requires_owner_privileges".to_owned(),
            Self::RequiresRightPhrase => "requires_right_phrase".to_owned(),
            Self::GroupCallFact => "group_call_fact".to_owned(),
            Self::GroupCallMessageFact => "group_call_message_fact".to_owned(),
            Self::SupergroupFullInfoFact => "supergroup_full_info_fact".to_owned(),
            Self::MessagePropertiesFact => "message_properties_fact".to_owned(),
            Self::ChatBoostReference => "chat_boost_reference".to_owned(),
            Self::BoostLevelPhrase => "boost_level_phrase".to_owned(),
            Self::CanFieldReference => "can_field_reference".to_owned(),
            Self::IsFieldReference => "is_field_reference".to_owned(),
            Self::UserIsAdministrator => "user_is_administrator".to_owned(),
            Self::MustBeAdministrator => "must_be_administrator".to_owned(),
            Self::MustHaveAdministratorPrivileges => {
                "must_have_administrator_privileges".to_owned()
            }
            Self::NamedRight(right) => format!("named_right:{}", right.as_str()),
            Self::OnlyByAdministrator => "only_by_administrator".to_owned(),
            Self::OnlyIfAdministrator => "only_if_administrator".to_owned(),
            Self::AdministratorPrivilegesMayBeRequired => {
                "administrator_privileges_may_be_required".to_owned()
            }
            Self::AdministratorRightsMayBeRequired => {
                "administrator_rights_may_be_required".to_owned()
            }
            Self::OptionGate => "option_gate".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct RuntimeSignalKey {
    source: RuntimeSignalSource,
    family: RuntimeSignalFamily,
}

impl RuntimeSignalKey {
    fn source(&self) -> &RuntimeSignalSource {
        &self.source
    }

    fn family(&self) -> RuntimeSignalFamily {
        self.family
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeferredSignalLane {
    UnclassifiedDescription,
    InputPrerequisite,
    RetryCondition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NonGateReason {
    ChatBoostVocabulary,
    GroupCallParticipantUnmutePolicy,
    SupergroupFullInfoCrossTokenWording,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuntimeSignalDisposition {
    ConsumedByRuntimeRequirements,
    Deferred(DeferredSignalLane),
    NotRuntimeGate(NonGateReason),
}

#[cfg(test)]
impl RuntimeSignalDisposition {
    fn canonical_name(self) -> &'static str {
        match self {
            Self::ConsumedByRuntimeRequirements => "consumed_by_runtime_requirements",
            Self::Deferred(DeferredSignalLane::UnclassifiedDescription) => {
                "deferred:unclassified_description"
            }
            Self::Deferred(DeferredSignalLane::InputPrerequisite) => "deferred:input_prerequisite",
            Self::Deferred(DeferredSignalLane::RetryCondition) => "deferred:retry_condition",
            Self::NotRuntimeGate(NonGateReason::ChatBoostVocabulary) => {
                "not_runtime_gate:chat_boost_vocabulary"
            }
            Self::NotRuntimeGate(NonGateReason::GroupCallParticipantUnmutePolicy) => {
                "not_runtime_gate:group_call_participant_unmute_policy"
            }
            Self::NotRuntimeGate(NonGateReason::SupergroupFullInfoCrossTokenWording) => {
                "not_runtime_gate:supergroup_full_info_cross_token_wording"
            }
        }
    }
}

fn documented_runtime_signal_keys(
    method: &Definition,
) -> Result<BTreeSet<RuntimeSignalKey>, CapabilityGenerationError> {
    let mut keys = BTreeSet::new();
    let mut sources = BTreeSet::new();
    for tag in method.documentation().tags() {
        let families = runtime_signal_families(&normalized_text(tag.value()));
        if families.is_empty() {
            continue;
        }
        let source = if tag.name() == "description" {
            RuntimeSignalSource::Description
        } else if field_type(method, tag.name()).is_some() {
            RuntimeSignalSource::Argument(
                ArgumentRef::try_from(tag.name())
                    .map_err(CapabilityGenerationError::from_model_value)?,
            )
        } else {
            return Err(unsupported_runtime_documentation(
                method,
                "runtime signal belongs to neither @description nor a method argument",
            ));
        };
        if !sources.insert(source.clone()) {
            return Err(unsupported_runtime_documentation(
                method,
                "runtime signal source tag is duplicated",
            ));
        }
        keys.extend(families.into_iter().map(|family| RuntimeSignalKey {
            source: source.clone(),
            family,
        }));
    }
    Ok(keys)
}

fn documented_runtime_signal_dispositions(
    method: &Definition,
) -> Result<Vec<(RuntimeSignalKey, RuntimeSignalDisposition)>, CapabilityGenerationError> {
    let description = normalized_text(&method_description(method));
    let mut consumed = reviewed_runtime_contract(method.name(), &description)
        .map(ReviewedRuntimeContract::consumed_signal_keys)
        .unwrap_or_default();
    if let Some(contract) = reviewed_message_capability_contract(method)? {
        consumed.extend(contract.consumed_signal_keys()?);
    }
    if let Some(contract) = reviewed_group_call_contract(method)? {
        consumed.extend(contract.consumed_signal_keys());
    }
    if reviewed_message_moderation_contract(method)?.is_some() {
        consumed.extend(message_moderation_consumed_signal_keys());
    }
    if let Some(contract) = reviewed_supergroup_full_info_contract(method)? {
        consumed.extend(contract.consumed_signal_keys());
    }
    if let Some(contract) = reviewed_runtime_boolean_option_contract(method)? {
        consumed.extend(contract.consumed_signal_keys());
    }
    if reviewed_chat_boost_contract(method)?.is_some() {
        consumed.extend(chat_administrator_consumed_signal_keys());
    }
    if reviewed_supergroup_username_contract(method)?.is_some() {
        consumed.extend(supergroup_username_consumed_signal_keys());
    }
    if let Some(contract) = reviewed_chat_invite_link_contract(method)? {
        consumed.extend(chat_invite_link_consumed_signal_keys(contract));
    }
    if reviewed_chat_event_log_contract(method)?.is_some() {
        consumed.extend(chat_administrator_consumed_signal_keys());
    }
    if let Some(contract) = reviewed_chat_setting_contract(method)? {
        consumed.extend(chat_setting_consumed_signal_keys(contract));
    }
    if let Some(contract) = reviewed_video_chat_contract(method)? {
        consumed.extend(video_chat_consumed_signal_keys(contract));
    }
    runtime_signal_dispositions_with_consumed(method, &consumed)
}

fn runtime_signal_dispositions_with_consumed(
    method: &Definition,
    consumed: &BTreeSet<RuntimeSignalKey>,
) -> Result<Vec<(RuntimeSignalKey, RuntimeSignalDisposition)>, CapabilityGenerationError> {
    documented_runtime_signal_keys(method).map(|keys| {
        keys.into_iter()
            .map(|key| {
                let disposition = if let Some(reason) = terminal_non_gate_reason(method, &key) {
                    RuntimeSignalDisposition::NotRuntimeGate(reason)
                } else if consumed.contains(&key) {
                    RuntimeSignalDisposition::ConsumedByRuntimeRequirements
                } else if is_retry_signal(method, &key) {
                    RuntimeSignalDisposition::Deferred(DeferredSignalLane::RetryCondition)
                } else if matches!(key.source, RuntimeSignalSource::Argument(_)) {
                    RuntimeSignalDisposition::Deferred(DeferredSignalLane::InputPrerequisite)
                } else {
                    RuntimeSignalDisposition::Deferred(DeferredSignalLane::UnclassifiedDescription)
                };
                (key, disposition)
            })
            .collect()
    })
}

fn terminal_non_gate_reason(method: &Definition, key: &RuntimeSignalKey) -> Option<NonGateReason> {
    match (method.name(), key.source(), key.family()) {
        (
            "toggleVideoChatMuteNewParticipants",
            RuntimeSignalSource::Description,
            RuntimeSignalFamily::OnlyByAdministrator,
        ) => signal_source_has_exact_text(
            method,
            key.source(),
            "toggles whether new participants of a video chat can be unmuted only by administrators of the video chat. requires groupcall.can_toggle_mute_new_participants right",
        )
        .then_some(NonGateReason::GroupCallParticipantUnmutePolicy),
        (
            "toggleSupergroupHasHiddenMembers",
            RuntimeSignalSource::Description,
            RuntimeSignalFamily::OnlyIfAdministrator,
        ) => signal_source_has_exact_text(
            method,
            key.source(),
            TOGGLE_SUPERGROUP_HAS_HIDDEN_MEMBERS_DESCRIPTION,
        )
        .then_some(NonGateReason::SupergroupFullInfoCrossTokenWording),
        (
            "getSupergroupMembers",
            RuntimeSignalSource::Description,
            RuntimeSignalFamily::OnlyIfAdministrator,
        ) => signal_source_has_exact_text(
            method,
            key.source(),
            GET_SUPERGROUP_MEMBERS_DESCRIPTION,
        )
        .then_some(NonGateReason::SupergroupFullInfoCrossTokenWording),
        (
            "getChatBoostFeatures",
            RuntimeSignalSource::Description,
            RuntimeSignalFamily::BoostLevelPhrase,
        ) => signal_source_has_exact_text(
            method,
            key.source(),
            "returns the list of features available for different chat boost levels. this is an offline method",
        )
        .then_some(NonGateReason::ChatBoostVocabulary),
        (
            "getChatBoostLevelFeatures",
            RuntimeSignalSource::Description,
            RuntimeSignalFamily::BoostLevelPhrase,
        ) => signal_source_has_exact_text(
            method,
            key.source(),
            "returns the list of features available on the specific chat boost level. this is an offline method",
        )
        .then_some(NonGateReason::ChatBoostVocabulary),
        (
            "getChatBoostLevelFeatures",
            RuntimeSignalSource::Argument(argument),
            RuntimeSignalFamily::BoostLevelPhrase,
        ) => (argument.as_str() == "level"
            && signal_source_has_exact_text(method, key.source(), "chat boost level"))
        .then_some(NonGateReason::ChatBoostVocabulary),
        (
            "getChatBoostLinkInfo",
            RuntimeSignalSource::Description,
            RuntimeSignalFamily::ChatBoostReference,
        ) => signal_source_has_exact_text(
            method,
            key.source(),
            "returns information about a link to boost a chat. can be called for any internal link of the type internallinktypechatboost",
        )
        .then_some(NonGateReason::ChatBoostVocabulary),
        _ => None,
    }
}

fn is_retry_signal(method: &Definition, key: &RuntimeSignalKey) -> bool {
    if !matches!(
        (key.source(), key.family()),
        (
            RuntimeSignalSource::Description,
            RuntimeSignalFamily::CanFieldReference,
        )
    ) {
        return false;
    }
    matches!(
        (
            method.name(),
            normalized_signal_source_text(method, key.source()).as_deref()
        ),
        (
            "resendMessages",
            Some(
                "resends messages which failed to send. can be called only for messages for which messagesendingstatefailed.can_retry is true and after specified in messagesendingstatefailed.retry_after time passed. if a message is re-sent, the corresponding failed to send message is deleted. returns the sent messages in the same order as the message identifiers passed in message_ids. if a message can't be re-sent, null will be returned instead of the message"
            )
        ) | (
            "readdQuickReplyShortcutMessages",
            Some(
                "readds quick reply messages which failed to add. can be called only for messages for which messagesendingstatefailed.can_retry is true and after specified in messagesendingstatefailed.retry_after time passed. if a message is readded, the corresponding failed to send message is deleted. returns the sent messages in the same order as the message identifiers passed in message_ids. if a message can't be readded, null will be returned instead of the message"
            )
        )
    )
}

fn normalized_signal_source_text(
    method: &Definition,
    source: &RuntimeSignalSource,
) -> Option<String> {
    let tag_name = match source {
        RuntimeSignalSource::Description => "description",
        RuntimeSignalSource::Argument(argument) => argument.as_str(),
    };
    method
        .documentation()
        .tags()
        .iter()
        .find(|tag| tag.name() == tag_name)
        .map(|tag| normalized_text(tag.value()))
}

fn signal_source_has_exact_text(
    method: &Definition,
    source: &RuntimeSignalSource,
    expected: &str,
) -> bool {
    let tag_name = match source {
        RuntimeSignalSource::Description => "description",
        RuntimeSignalSource::Argument(argument) => argument.as_str(),
    };
    let mut tags = method
        .documentation()
        .tags()
        .iter()
        .filter(|tag| tag.name() == tag_name);
    let Some(tag) = tags.next() else {
        return false;
    };
    tags.next().is_none() && normalized_text(tag.value()) == expected
}

fn runtime_signal_families(value: &str) -> BTreeSet<RuntimeSignalFamily> {
    let mut families = BTreeSet::new();
    if contains_word_sequence(value, "administrator right") {
        families.insert(RuntimeSignalFamily::AdministratorRightPhrase);
    }
    if contains_word_sequence(value, "member right") {
        families.insert(RuntimeSignalFamily::MemberRightPhrase);
    }
    if value.contains("requires administrator") {
        families.insert(RuntimeSignalFamily::RequiresAdministrator);
    }
    if value.contains("requires owner privileges") {
        families.insert(RuntimeSignalFamily::RequiresOwnerPrivileges);
    }
    if value.contains("requires ") && contains_word_sequence(value, "right") {
        families.insert(RuntimeSignalFamily::RequiresRightPhrase);
    }
    if value.contains("requires groupcall.") {
        families.insert(RuntimeSignalFamily::GroupCallFact);
    }
    if value.contains("requires groupcallmessage.") {
        families.insert(RuntimeSignalFamily::GroupCallMessageFact);
    }
    if value.contains("supergroupfullinfo.") {
        families.insert(RuntimeSignalFamily::SupergroupFullInfoFact);
    }
    if value.contains("messageproperties.") {
        families.insert(RuntimeSignalFamily::MessagePropertiesFact);
    }
    if value.contains("chatboost") {
        families.insert(RuntimeSignalFamily::ChatBoostReference);
    }
    if value.contains("boost level") {
        families.insert(RuntimeSignalFamily::BoostLevelPhrase);
    }
    if value.contains(".can_") {
        families.insert(RuntimeSignalFamily::CanFieldReference);
    }
    if value.contains(".is_") {
        families.insert(RuntimeSignalFamily::IsFieldReference);
    }
    if value.contains("user is an administrator") {
        families.insert(RuntimeSignalFamily::UserIsAdministrator);
    }
    if value.contains("must be an administrator") {
        families.insert(RuntimeSignalFamily::MustBeAdministrator);
    }
    if value.contains("must have administrator privileges") {
        families.insert(RuntimeSignalFamily::MustHaveAdministratorPrivileges);
    }
    for right in ChatAdministratorRight::ALL {
        if value.contains(&format!("{} right", right.as_str())) {
            families.insert(RuntimeSignalFamily::NamedRight(right));
        }
    }
    if value.contains("only by ") && value.contains("administrator") {
        families.insert(RuntimeSignalFamily::OnlyByAdministrator);
    }
    if value.contains("only if ") && value.contains("administrator") {
        families.insert(RuntimeSignalFamily::OnlyIfAdministrator);
    }
    if value.contains("administrator privileges may be required") {
        families.insert(RuntimeSignalFamily::AdministratorPrivilegesMayBeRequired);
    }
    if value.contains("administrator rights may be required") {
        families.insert(RuntimeSignalFamily::AdministratorRightsMayBeRequired);
    }
    if value.contains("only if getoption(") {
        families.insert(RuntimeSignalFamily::OptionGate);
    }
    families
}

#[cfg(test)]
fn has_runtime_gate_signal(value: &str) -> bool {
    !runtime_signal_families(value).is_empty()
}

fn contains_word_sequence(value: &str, phrase: &str) -> bool {
    value.match_indices(phrase).any(|(start, _)| {
        let before = value[..start].chars().next_back();
        let after = value[start + phrase.len()..].chars().next();
        before.is_none_or(|character| !is_identifier_character(character))
            && after.is_none_or(|character| !is_identifier_character(character))
    })
}

fn is_identifier_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

fn unsupported_runtime_documentation(
    method: &Definition,
    reason: &str,
) -> CapabilityGenerationError {
    CapabilityGenerationError::new(
        CapabilityGenerationErrorKind::SchemaDrift,
        format!(
            "unsupported runtime documentation for {:?}: {reason}",
            method.name()
        ),
    )
}

fn normalized_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn validate_documented_parameter_notices(
    method: &Definition,
    descriptor: &CapabilityDescriptor,
) -> Result<(), CapabilityGenerationError> {
    let actual = descriptor
        .parameter_notices()
        .iter()
        .map(|notice| (notice.parameter().as_str(), notice.gate()))
        .collect::<BTreeSet<_>>();
    let mut expected_notices = BTreeSet::new();
    for tag in method.documentation().tags() {
        if field_type(method, tag.name()).is_none() {
            continue;
        }
        let value = tag.value().to_ascii_lowercase();
        let selects_dc =
            value.contains("use telegram test environment instead of the production environment");
        let mut documented_gates = Vec::new();
        if value.contains("only to bot accounts")
            || value.contains("; for bots only")
            || value.ends_with("for bots only")
            || value.contains("; bots only")
        {
            documented_gates.push(ParameterGate::Account(AccountKind::Bot));
        }
        if value.contains("premium of the current account")
            || value.contains("for telegram premium users only")
            || value.contains("only by telegram premium users")
            || value.contains("only for telegram premium users")
            || value.contains("only for premium users")
            || value.contains("for telegram premium users,")
            || value.contains("telegram premium users can")
            || value.contains("requires telegram premium")
            || value.contains("with telegram premium")
        {
            documented_gates.push(ParameterGate::CurrentAccountEntitlement(
                CurrentAccountEntitlement::Premium,
            ));
        }
        if value.contains("business of the current account")
            || value.contains("require telegram business subscription")
            || value.contains("requires telegram business subscription")
        {
            documented_gates.push(ParameterGate::CurrentAccountEntitlement(
                CurrentAccountEntitlement::Business,
            ));
        }
        if value.contains("official mobile applications") {
            documented_gates.push(ParameterGate::Application(
                ApplicationRequirement::OfficialMobile,
            ));
        }
        if value.contains("official telegram apps") || value.contains("official applications") {
            documented_gates.push(ParameterGate::Application(ApplicationRequirement::Official));
        }
        if !selects_dc
            && (value.contains("production dc") || value.contains("production environment"))
        {
            documented_gates.push(ParameterGate::DcEnvironment(DcEnvironment::Production));
        }
        if !selects_dc && (value.contains("test dc") || value.contains("telegram test environment"))
        {
            documented_gates.push(ParameterGate::DcEnvironment(DcEnvironment::Test));
        }
        for gate in documented_gates {
            if !method_documentation_implies_gate(method, gate) {
                expected_notices.insert((tag.name(), gate));
            }
        }
    }

    if let Some((parameter, gate)) = expected_notices.difference(&actual).next() {
        return Err(CapabilityGenerationError::invalid_policy(format!(
            "missing documented parameter notice for {:?}.{parameter}: {gate:?}",
            method.name()
        )));
    }
    if let Some((parameter, gate)) = actual.difference(&expected_notices).next() {
        return Err(CapabilityGenerationError::invalid_policy(format!(
            "undocumented parameter notice for {:?}.{parameter}: {gate:?}",
            method.name()
        )));
    }
    Ok(())
}

fn method_documentation_implies_gate(method: &Definition, gate: ParameterGate) -> bool {
    let description = method_description(method).to_ascii_lowercase();
    match gate {
        ParameterGate::Account(AccountKind::Bot) => description.contains("for bots only"),
        ParameterGate::Account(AccountKind::RegularUser) => false,
        ParameterGate::CurrentAccountEntitlement(CurrentAccountEntitlement::Premium) => {
            description.contains("for telegram premium users only")
                || description.contains("current premium users only")
        }
        ParameterGate::CurrentAccountEntitlement(CurrentAccountEntitlement::Business) => {
            description.contains("requires telegram business subscription")
                || description.contains("current business users only")
        }
        ParameterGate::Application(ApplicationRequirement::Any) => false,
        ParameterGate::Application(ApplicationRequirement::Official) => {
            description.contains("for official telegram apps only")
                || description.contains("official applications only")
                || description.contains("official mobile applications only")
        }
        ParameterGate::Application(ApplicationRequirement::OfficialMobile) => {
            description.contains("official mobile applications only")
        }
        ParameterGate::DcEnvironment(DcEnvironment::Production) => {
            description.contains("production dc only")
        }
        ParameterGate::DcEnvironment(DcEnvironment::Test) => description.contains("test dc only"),
    }
}

fn method_description(method: &Definition) -> String {
    method
        .documentation()
        .tags()
        .iter()
        .filter(|tag| tag.name() == "description")
        .map(|tag| tag.value())
        .collect::<Vec<_>>()
        .join("\n")
}

fn method_documentation_text(method: &Definition) -> String {
    method
        .documentation()
        .tags()
        .iter()
        .map(|tag| tag.value())
        .collect::<Vec<_>>()
        .join("\n")
}

fn quoted_values(value: &str) -> BTreeSet<&str> {
    let mut values = BTreeSet::new();
    let mut remainder = value;
    while let Some((_, after_open)) = remainder.split_once('"') {
        let Some((quoted, after_close)) = after_open.split_once('"') else {
            break;
        };
        values.insert(quoted);
        remainder = after_close;
    }
    values
}

fn documentation_sha256(method: &Definition) -> String {
    let mut hasher = Sha256::new();
    for line in method.documentation().raw_lines() {
        hasher.update(line.as_bytes());
        hasher.update(b"\n");
    }
    digest_hex(hasher.finalize())
}

fn validate_rationale(value: &str) -> Result<(), CapabilityGenerationError> {
    if value.trim().is_empty() || value != value.trim() || value.len() > MAX_RATIONALE_BYTES {
        return Err(CapabilityGenerationError::invalid_policy(format!(
            "method rationale must be trimmed and contain 1..={MAX_RATIONALE_BYTES} bytes"
        )));
    }
    Ok(())
}

fn validate_hash(name: &str, value: &str) -> Result<(), CapabilityGenerationError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(CapabilityGenerationError::invalid_policy(format!(
            "{name} must be 64 lowercase hexadecimal characters"
        )));
    }
    Ok(())
}

fn enforce_cap(name: &str, actual: usize, limit: usize) -> Result<(), CapabilityGenerationError> {
    if actual > limit {
        return Err(CapabilityGenerationError::resource_limit(format!(
            "{name} is {actual} bytes, exceeding the {limit}-byte cap"
        )));
    }
    Ok(())
}

fn compact_json<T: Serialize>(value: &T, name: &str) -> Result<Vec<u8>, CapabilityGenerationError> {
    serde_json::to_vec(value).map_err(|error| {
        CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::Serialization,
            format!("cannot serialize canonical {name}: {error}"),
        )
    })
}

fn serialize_pretty_with_limit<T: Serialize>(
    value: &T,
    limit: usize,
) -> Result<Vec<u8>, CapabilityGenerationError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::Serialization,
            format!("cannot serialize capability manifest: {error}"),
        )
    })?;
    bytes.push(b'\n');
    if bytes.len() > limit {
        return Err(CapabilityGenerationError::resource_limit(format!(
            "generated capability manifest is {} bytes, exceeding the {limit}-byte cap",
            bytes.len()
        )));
    }
    Ok(bytes)
}

fn sha256_hex(bytes: &[u8]) -> String {
    digest_hex(Sha256::digest(bytes))
}

fn engine_source_sha256() -> String {
    let mut hasher = Sha256::new();
    for (path, bytes) in [
        ("capability.rs", include_bytes!("capability.rs").as_slice()),
        (
            "capability/chat_boosts.rs",
            include_bytes!("capability/chat_boosts.rs").as_slice(),
        ),
        (
            "capability/chat_event_logs.rs",
            include_bytes!("capability/chat_event_logs.rs").as_slice(),
        ),
        (
            "capability/chat_invite_links.rs",
            include_bytes!("capability/chat_invite_links.rs").as_slice(),
        ),
        (
            "capability/chat_settings.rs",
            include_bytes!("capability/chat_settings.rs").as_slice(),
        ),
        (
            "capability/message_moderation.rs",
            include_bytes!("capability/message_moderation.rs").as_slice(),
        ),
        (
            "capability/supergroup_usernames.rs",
            include_bytes!("capability/supergroup_usernames.rs").as_slice(),
        ),
        (
            "capability/video_chats.rs",
            include_bytes!("capability/video_chats.rs").as_slice(),
        ),
        (
            "telegram-core/method_capability.rs",
            include_bytes!("../../../crates/telegram-core/src/method_capability.rs").as_slice(),
        ),
        (
            "telegram-core/schema.rs",
            include_bytes!("../../../crates/telegram-core/src/schema.rs").as_slice(),
        ),
    ] {
        hasher.update(path.as_bytes());
        hasher.update([0]);
        hasher.update(bytes);
        hasher.update(b"\n");
    }
    digest_hex(hasher.finalize())
}

fn digest_hex(digest: impl AsRef<[u8]>) -> String {
    let mut encoded = String::with_capacity(digest.as_ref().len() * 2);
    for byte in digest.as_ref() {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CapabilityPolicyDto {
    format_version: u32,
    schema_sha256: String,
    methods: Vec<MethodPolicyDto>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MethodPolicyDto {
    method: String,
    signature_sha256: String,
    documentation_sha256: String,
    synchronous: SynchronousDto,
    ready_accounts: Vec<String>,
    authorization_states: Vec<String>,
    current_account_entitlements: Vec<String>,
    application: String,
    dc_environments: Vec<String>,
    runtime_requirements: RuntimeRequirementsDto,
    parameter_notices: Vec<ParameterNoticeDto>,
    rationale: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum SynchronousDto {
    Never,
    Always,
    StringParameterValues {
        parameter: String,
        values: Vec<String>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum RuntimeRequirementsDto {
    Always,
    AnyOf { clauses: Vec<RequirementClauseDto> },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequirementClauseDto {
    all_of: Vec<RuntimeRequirementDto>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum RuntimeRequirementDto {
    ChatKind {
        target_argument: String,
        value: String,
    },
    SupergroupFlag {
        target_argument: String,
        flag: String,
        value: bool,
    },
    ChatAdministrator {
        target_argument: String,
    },
    ChatAdministratorRight {
        target_argument: String,
        right: String,
    },
    ChatMemberRight {
        target_argument: String,
        right: String,
    },
    ChatOwner {
        target_argument: String,
    },
    TopicCreator {
        target_argument: String,
        topic_argument: String,
    },
    BusinessConnectionEnabled {
        connection_argument: String,
    },
    BusinessConnectionRight {
        connection_argument: String,
        right: String,
    },
    MessageCapability {
        subject: MessageSubjectDto,
        capability: String,
    },
    GroupCallKind {
        group_call_argument: String,
        value: String,
    },
    GroupCallProperty {
        group_call_argument: String,
        property: String,
    },
    GroupCallMessageCapability {
        subject: GroupCallMessageSubjectDto,
        capability: String,
    },
    SupergroupFullInfoProperty {
        target_argument: String,
        property: String,
    },
    BooleanOptionEnabled {
        option: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum MessageSubjectDto {
    One {
        chat_argument: String,
        message_argument: String,
    },
    Each {
        chat_argument: String,
        message_argument: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum GroupCallMessageSubjectDto {
    Each {
        group_call_argument: String,
        message_argument: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ParameterNoticeDto {
    parameter: String,
    gate: ParameterGateDto,
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum ParameterGateDto {
    Account(String),
    CurrentAccountEntitlement(String),
    Application(String),
    DcEnvironment(String),
}

#[derive(Serialize)]
struct CanonicalPolicy<'a> {
    format_version: u32,
    schema_sha256: &'a str,
    methods: &'a [CanonicalMethodRow],
}

#[derive(Serialize)]
struct GeneratedManifest {
    format_version: u32,
    generated_by: &'static str,
    engine_source_sha256: String,
    schema: SchemaEvidence,
    policy: PolicyEvidence,
    counts: Counts,
    mapping_sha256: String,
    methods: Vec<CanonicalMethodRow>,
}

#[derive(Serialize)]
struct SchemaEvidence {
    sha256: String,
    methods: usize,
    authorization_states: usize,
}

#[derive(Serialize)]
struct PolicyEvidence {
    semantic_sha256: String,
}

#[derive(Serialize)]
struct Counts {
    schema_methods: usize,
    capability_methods: usize,
}

#[derive(Serialize)]
struct CanonicalMethodRow {
    method: String,
    signature_sha256: String,
    documentation_sha256: String,
    synchronous: CanonicalSynchronous,
    ready_accounts: Vec<&'static str>,
    authorization_states: Vec<&'static str>,
    current_account_entitlements: Vec<&'static str>,
    application: &'static str,
    dc_environments: Vec<&'static str>,
    runtime_requirements: CanonicalRuntimeRequirements,
    parameter_notices: Vec<CanonicalParameterNotice>,
    rationale: String,
}

impl CanonicalMethodRow {
    #[allow(clippy::too_many_arguments)]
    fn from_descriptor(
        method: String,
        signature_sha256: String,
        documentation_sha256: String,
        descriptor: CapabilityDescriptor,
        rationale: String,
    ) -> Self {
        Self {
            method,
            signature_sha256,
            documentation_sha256,
            synchronous: CanonicalSynchronous::from_domain(descriptor.synchronous()),
            ready_accounts: descriptor
                .ready_accounts()
                .iter()
                .map(|value| value.as_str())
                .collect(),
            authorization_states: descriptor
                .authorization_states()
                .iter()
                .map(|value| value.as_str())
                .collect(),
            current_account_entitlements: descriptor
                .current_account_entitlements()
                .iter()
                .map(|value| value.as_str())
                .collect(),
            application: descriptor.application().as_str(),
            dc_environments: descriptor
                .dc_environments()
                .iter()
                .map(|value| value.as_str())
                .collect(),
            runtime_requirements: CanonicalRuntimeRequirements::from_domain(
                descriptor.runtime_requirements(),
            ),
            parameter_notices: descriptor
                .parameter_notices()
                .iter()
                .map(CanonicalParameterNotice::from_domain)
                .collect(),
            rationale,
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CanonicalSynchronous {
    Never,
    Always,
    StringParameterValues {
        parameter: String,
        values: Vec<String>,
    },
}

impl CanonicalSynchronous {
    fn from_domain(value: &SynchronousCapability) -> Self {
        match value {
            SynchronousCapability::Never => Self::Never,
            SynchronousCapability::Always => Self::Always,
            SynchronousCapability::StringParameterValues(condition) => {
                Self::StringParameterValues {
                    parameter: condition.parameter().as_str().to_owned(),
                    values: condition
                        .values()
                        .iter()
                        .map(|value| value.as_str().to_owned())
                        .collect(),
                }
            }
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CanonicalRuntimeRequirements {
    Always,
    AnyOf { clauses: Vec<CanonicalClause> },
}

impl CanonicalRuntimeRequirements {
    fn from_domain(value: &RequirementAlternatives) -> Self {
        if value.is_always() {
            Self::Always
        } else {
            Self::AnyOf {
                clauses: value
                    .clauses()
                    .iter()
                    .map(|clause| CanonicalClause {
                        all_of: clause
                            .iter()
                            .map(CanonicalRuntimeRequirement::from_domain)
                            .collect(),
                    })
                    .collect(),
            }
        }
    }
}

#[derive(Serialize)]
struct CanonicalClause {
    all_of: Vec<CanonicalRuntimeRequirement>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CanonicalRuntimeRequirement {
    ChatKind {
        target: CanonicalChatTarget,
        value: &'static str,
    },
    SupergroupFlag {
        target: CanonicalChatTarget,
        flag: &'static str,
        value: bool,
    },
    ChatAdministrator {
        target: CanonicalChatTarget,
    },
    ChatAdministratorRight {
        target: CanonicalChatTarget,
        right: &'static str,
    },
    ChatMemberRight {
        target: CanonicalChatTarget,
        right: &'static str,
    },
    ChatOwner {
        target: CanonicalChatTarget,
    },
    TopicCreator {
        target: CanonicalChatTarget,
        topic_argument: String,
    },
    BusinessConnectionEnabled {
        connection_argument: String,
    },
    BusinessConnectionRight {
        connection_argument: String,
        right: &'static str,
    },
    MessageCapability {
        subject: CanonicalMessageSubject,
        capability: &'static str,
    },
    GroupCallKind {
        group_call_argument: String,
        value: &'static str,
    },
    GroupCallProperty {
        group_call_argument: String,
        property: &'static str,
    },
    GroupCallMessageCapability {
        subject: CanonicalGroupCallMessageSubject,
        capability: &'static str,
    },
    SupergroupFullInfoProperty {
        target: CanonicalChatTarget,
        property: &'static str,
    },
    BooleanOptionEnabled {
        option: &'static str,
    },
}

impl CanonicalRuntimeRequirement {
    fn from_domain(value: &RuntimeRequirement) -> Self {
        match value {
            RuntimeRequirement::ChatKind(condition) => Self::ChatKind {
                target: CanonicalChatTarget::from_domain(condition.target()),
                value: condition.kind().as_str(),
            },
            RuntimeRequirement::SupergroupFlag(condition) => Self::SupergroupFlag {
                target: CanonicalChatTarget::from_domain(condition.target()),
                flag: condition.flag().as_str(),
                value: condition.value(),
            },
            RuntimeRequirement::ChatAdministrator { target } => Self::ChatAdministrator {
                target: CanonicalChatTarget::from_domain(target),
            },
            RuntimeRequirement::ChatAdministratorRight { target, right } => {
                Self::ChatAdministratorRight {
                    target: CanonicalChatTarget::from_domain(target),
                    right: right.as_str(),
                }
            }
            RuntimeRequirement::ChatMemberRight { target, right } => Self::ChatMemberRight {
                target: CanonicalChatTarget::from_domain(target),
                right: right.as_str(),
            },
            RuntimeRequirement::ChatOwner { target } => Self::ChatOwner {
                target: CanonicalChatTarget::from_domain(target),
            },
            RuntimeRequirement::TopicCreator { target, topic } => Self::TopicCreator {
                target: CanonicalChatTarget::from_domain(target),
                topic_argument: topic.argument().as_str().to_owned(),
            },
            RuntimeRequirement::BusinessConnectionEnabled { connection } => {
                Self::BusinessConnectionEnabled {
                    connection_argument: connection.argument().as_str().to_owned(),
                }
            }
            RuntimeRequirement::BusinessConnectionRight { connection, right } => {
                Self::BusinessConnectionRight {
                    connection_argument: connection.argument().as_str().to_owned(),
                    right: right.as_str(),
                }
            }
            RuntimeRequirement::MessageCapability {
                subject,
                capability,
            } => Self::MessageCapability {
                subject: CanonicalMessageSubject::from_domain(subject),
                capability: capability.as_str(),
            },
            RuntimeRequirement::GroupCallKind(condition) => Self::GroupCallKind {
                group_call_argument: condition.group_call().argument().as_str().to_owned(),
                value: condition.kind().as_str(),
            },
            RuntimeRequirement::GroupCallProperty {
                group_call,
                property,
            } => Self::GroupCallProperty {
                group_call_argument: group_call.argument().as_str().to_owned(),
                property: property.as_str(),
            },
            RuntimeRequirement::GroupCallMessageCapability {
                subject,
                capability,
            } => Self::GroupCallMessageCapability {
                subject: CanonicalGroupCallMessageSubject::from_domain(subject),
                capability: capability.as_str(),
            },
            RuntimeRequirement::SupergroupFullInfoProperty { target, property } => {
                Self::SupergroupFullInfoProperty {
                    target: CanonicalChatTarget::from_domain(target),
                    property: property.as_str(),
                }
            }
            RuntimeRequirement::BooleanOptionEnabled { option } => Self::BooleanOptionEnabled {
                option: option.as_str(),
            },
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CanonicalGroupCallMessageSubject {
    Each {
        group_call_argument: String,
        message_argument: String,
    },
}

impl CanonicalGroupCallMessageSubject {
    fn from_domain(value: &GroupCallMessageSubjectRef) -> Self {
        match value {
            GroupCallMessageSubjectRef::Each {
                group_call,
                messages,
            } => Self::Each {
                group_call_argument: group_call.argument().as_str().to_owned(),
                message_argument: messages.argument().as_str().to_owned(),
            },
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CanonicalMessageSubject {
    One {
        chat: CanonicalChatTarget,
        message_argument: String,
    },
    Each {
        chat: CanonicalChatTarget,
        message_argument: String,
    },
}

impl CanonicalMessageSubject {
    fn from_domain(value: &MessageSubjectRef) -> Self {
        match value {
            MessageSubjectRef::One { chat, message } => Self::One {
                chat: CanonicalChatTarget::from_domain(chat),
                message_argument: message.argument().as_str().to_owned(),
            },
            MessageSubjectRef::Each { chat, messages } => Self::Each {
                chat: CanonicalChatTarget::from_domain(chat),
                message_argument: messages.argument().as_str().to_owned(),
            },
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CanonicalChatTarget {
    ChatId { argument: String },
    SupergroupId { argument: String },
}

impl CanonicalChatTarget {
    fn from_domain(value: &ChatTargetRef) -> Self {
        let argument = value.argument().as_str().to_owned();
        match value.kind() {
            ChatTargetKind::ChatId => Self::ChatId { argument },
            ChatTargetKind::SupergroupId => Self::SupergroupId { argument },
        }
    }
}

#[derive(Serialize)]
struct CanonicalParameterNotice {
    parameter: String,
    gate: CanonicalParameterGate,
}

impl CanonicalParameterNotice {
    fn from_domain(value: &ParameterCapabilityNotice) -> Self {
        Self {
            parameter: value.parameter().as_str().to_owned(),
            gate: CanonicalParameterGate::from_domain(value.gate()),
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum CanonicalParameterGate {
    Account(&'static str),
    CurrentAccountEntitlement(&'static str),
    Application(&'static str),
    DcEnvironment(&'static str),
}

impl CanonicalParameterGate {
    fn from_domain(value: ParameterGate) -> Self {
        match value {
            ParameterGate::Account(value) => Self::Account(value.as_str()),
            ParameterGate::CurrentAccountEntitlement(value) => {
                Self::CurrentAccountEntitlement(value.as_str())
            }
            ParameterGate::Application(value) => Self::Application(value.as_str()),
            ParameterGate::DcEnvironment(value) => Self::DcEnvironment(value.as_str()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityGenerationErrorKind {
    ResourceLimit,
    InvalidSchema,
    SchemaDrift,
    InvalidPolicy,
    Coverage,
    Serialization,
}

#[derive(Debug)]
pub struct CapabilityGenerationError {
    kind: CapabilityGenerationErrorKind,
    detail: String,
}

impl CapabilityGenerationError {
    fn new(kind: CapabilityGenerationErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }

    fn invalid_policy(detail: impl Into<String>) -> Self {
        Self::new(CapabilityGenerationErrorKind::InvalidPolicy, detail)
    }

    fn resource_limit(detail: impl Into<String>) -> Self {
        Self::new(CapabilityGenerationErrorKind::ResourceLimit, detail)
    }

    fn from_model_value(error: impl fmt::Display) -> Self {
        Self::invalid_policy(error.to_string())
    }

    pub fn kind(&self) -> CapabilityGenerationErrorKind {
        self.kind
    }
}

impl fmt::Display for CapabilityGenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for CapabilityGenerationError {}

#[cfg(test)]
mod tests;
