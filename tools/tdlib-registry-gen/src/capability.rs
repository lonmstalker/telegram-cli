//! Pure, bounded generation of schema-bound TDLib method capabilities.
//!
//! This module classifies static requirements. It never claims that a runtime
//! account currently satisfies them and it does not grant policy permission.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Write};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use telegram_core::feature::FeatureId;
use telegram_core::method_capability::{
    AccountKind, ApplicationRequirement, ArgumentRef, AuthorizationState, BusinessBotRight,
    BusinessConnectionRef, CapabilityDescriptor, ChatAdministratorRight, ChatKindCondition,
    ChatMemberRight, ChatTargetKind, ChatTargetRef, CurrentAccountEntitlement, DcEnvironment,
    ForumTopicRef, MAX_ATOMS_PER_METHOD, MAX_CLAUSES_PER_METHOD, MAX_PARAMETER_NOTICES_PER_METHOD,
    MAX_SYNCHRONOUS_VALUES_PER_METHOD, MessageCapability, MessageIdRef, MessageIdsRef,
    MessageSubjectRef, ParameterCapabilityNotice, ParameterGate, ParameterStringValue,
    RequirementAlternatives, ResolvedChatKind, RuntimeRequirement, SynchronousCapability,
};
use telegram_core::schema::{Definition, DefinitionKind, Parameter, Schema};

use crate::engine;

const FORMAT_VERSION: u32 = 3;
const MAX_MANIFEST_BYTES: usize = 64 * 1024;
const MAX_SCHEMA_BYTES: usize = 2 * 1024 * 1024;
const MAX_OWNER_POLICY_BYTES: usize = 4 * 1024 * 1024;
const MAX_CAPABILITY_POLICY_BYTES: usize = 4 * 1024 * 1024;
const MAX_OWNER_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_METHODS: usize = 2_048;
const MAX_RATIONALE_BYTES: usize = 1_024;

pub fn generate(
    manifest_bytes: &[u8],
    schema_bytes: &[u8],
    owner_policy_bytes: &[u8],
    capability_policy_bytes: &[u8],
) -> Result<Vec<u8>, CapabilityGenerationError> {
    enforce_cap("vendor manifest", manifest_bytes.len(), MAX_MANIFEST_BYTES)?;
    enforce_cap("TDLib schema", schema_bytes.len(), MAX_SCHEMA_BYTES)?;
    enforce_cap(
        "owner policy",
        owner_policy_bytes.len(),
        MAX_OWNER_POLICY_BYTES,
    )?;
    enforce_cap(
        "capability policy",
        capability_policy_bytes.len(),
        MAX_CAPABILITY_POLICY_BYTES,
    )?;

    // Recompute the owner manifest from its reviewed source. A committed or
    // caller-supplied generated artifact is deliberately not trusted.
    let owner_bytes =
        engine::generate(manifest_bytes, schema_bytes, owner_policy_bytes).map_err(|error| {
            CapabilityGenerationError::new(
                CapabilityGenerationErrorKind::OwnerGeneration,
                format!("owner generation failed ({:?}): {error}", error.kind()),
            )
        })?;
    enforce_cap("owner output", owner_bytes.len(), MAX_OWNER_OUTPUT_BYTES)?;
    let owner: OwnerManifest = serde_json::from_slice(&owner_bytes).map_err(|error| {
        CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::OwnerGeneration,
            format!("generated owner manifest is invalid: {error}"),
        )
    })?;

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

    let policy: CapabilityPolicyDto =
        serde_json::from_slice(capability_policy_bytes).map_err(|error| {
            CapabilityGenerationError::invalid_policy(format!("invalid capability policy: {error}"))
        })?;
    build_output(manifest_bytes, schema_bytes, schema, owner, policy)
}

fn build_output(
    manifest_bytes: &[u8],
    schema_bytes: &[u8],
    schema: Schema,
    owner: OwnerManifest,
    policy: CapabilityPolicyDto,
) -> Result<Vec<u8>, CapabilityGenerationError> {
    if policy.format_version != FORMAT_VERSION {
        return Err(CapabilityGenerationError::invalid_policy(format!(
            "unsupported capability policy format_version {}",
            policy.format_version
        )));
    }
    validate_hash("schema_sha256", &policy.schema_sha256)?;
    validate_hash("owner_mapping_sha256", &policy.owner_mapping_sha256)?;
    let actual_schema_hash = sha256_hex(schema_bytes);
    if policy.schema_sha256 != actual_schema_hash {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::SchemaDrift,
            "capability policy is bound to a different schema hash",
        ));
    }
    if policy.owner_mapping_sha256 != owner.mapping_sha256 {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::OwnerDrift,
            "capability policy is bound to a different owner mapping",
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
    let owners = owner_map(owner.methods, &methods)?;
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
        let owner_row = owners
            .get(method_name)
            .expect("owner method-set equality was checked");
        rows.push(build_method_row(definition, owner_row, policy_row)?);
    }

    let canonical_policy = CanonicalPolicy {
        format_version: FORMAT_VERSION,
        schema_sha256: &policy.schema_sha256,
        owner_mapping_sha256: &policy.owner_mapping_sha256,
        methods: &rows,
    };
    let semantic_policy = compact_json(&canonical_policy, "capability policy")?;
    let mapping_bytes = compact_json(&rows, "capability mapping")?;
    let output = GeneratedManifest {
        format_version: FORMAT_VERSION,
        generated_by: "tdlib-registry-gen/capability",
        engine_source_sha256: engine_source_sha256(),
        vendor_manifest_sha256: sha256_hex(manifest_bytes),
        schema: SchemaEvidence {
            sha256: policy.schema_sha256,
            methods: rows.len(),
            authorization_states: AuthorizationState::ALL.len(),
        },
        owner: OwnerEvidence {
            mapping_sha256: policy.owner_mapping_sha256,
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

fn owner_map(
    owner_rows: Vec<OwnerMethod>,
    schema_methods: &BTreeMap<&str, &Definition>,
) -> Result<BTreeMap<String, OwnerMethod>, CapabilityGenerationError> {
    let mut owners = BTreeMap::new();
    for row in owner_rows {
        if owners.insert(row.method.clone(), row).is_some() {
            return Err(CapabilityGenerationError::new(
                CapabilityGenerationErrorKind::OwnerGeneration,
                "generated owner manifest contains a duplicate method",
            ));
        }
    }
    if owners.keys().map(String::as_str).collect::<BTreeSet<_>>()
        != schema_methods.keys().copied().collect::<BTreeSet<_>>()
    {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::OwnerGeneration,
            "generated owner method set differs from schema",
        ));
    }
    Ok(owners)
}

fn build_method_row(
    method: &Definition,
    owner: &OwnerMethod,
    policy: MethodPolicyDto,
) -> Result<CanonicalMethodRow, CapabilityGenerationError> {
    validate_hash("method signature_sha256", &policy.signature_sha256)?;
    validate_hash("method documentation_sha256", &policy.documentation_sha256)?;
    validate_rationale(&policy.rationale)?;
    let signature_sha256 = sha256_hex(method.canonical_signature().as_bytes());
    if policy.signature_sha256 != signature_sha256 || owner.signature_sha256 != signature_sha256 {
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
    let feature_id = FeatureId::try_from(policy.feature_id.as_str()).map_err(|error| {
        CapabilityGenerationError::invalid_policy(format!(
            "invalid owner for {:?}: {error}",
            method.name()
        ))
    })?;
    if feature_id.as_str() != owner.feature_id {
        return Err(CapabilityGenerationError::new(
            CapabilityGenerationErrorKind::OwnerDrift,
            format!("stale owner evidence for {:?}", method.name()),
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
        feature_id.as_str(),
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
    );
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
    if runtime_contract.is_some() && message_contract.is_some() {
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

    if runtime_contract.is_none() && message_contract.is_none() {
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
    if consumed != expected_consumed {
        return Err(unsupported_runtime_documentation(
            method,
            "reviewed runtime requirements don't consume their exact signal set",
        ));
    }
    let clauses = if let Some(contract) = message_contract {
        documented_message_capability_clauses(method, contract)?
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
            ReviewedRuntimeContract::MemberRightInKinds { right, kinds } => {
                let target = documented_chat_target(method)?;
                chat_kind_clauses(&target, kinds, |target| {
                    RuntimeRequirement::ChatMemberRight {
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
            ReviewedRuntimeContract::ConditionalUnpin => {
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

#[derive(Clone, Copy)]
enum ReviewedRuntimeContract {
    AdministratorInKinds(&'static [ResolvedChatKind]),
    AdministratorRightInKinds {
        right: ChatAdministratorRight,
        kinds: &'static [ResolvedChatKind],
    },
    MemberRightInKinds {
        right: ChatMemberRight,
        kinds: &'static [ResolvedChatKind],
    },
    OwnerInKind(ResolvedChatKind),
    AdministratorOrTopicCreator {
        right: ChatAdministratorRight,
        kind: ResolvedChatKind,
    },
    ConditionalUnpin,
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
            Self::MemberRightInKinds { .. } => &[
                RuntimeSignalFamily::MemberRightPhrase,
                RuntimeSignalFamily::RequiresRightPhrase,
            ],
            Self::OwnerInKind(_) => &[RuntimeSignalFamily::RequiresOwnerPrivileges],
            Self::ConditionalUnpin => &[
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
            "deleteChatMessagesBySender",
            "deletes all messages sent by the specified message sender in a chat. supported only for supergroups; requires can_delete_messages administrator right",
        ) => Some(Contract::AdministratorRightInKinds {
            right: ChatAdministratorRight::CanDeleteMessages,
            kinds: &[ResolvedChatKind::Supergroup],
        }),
        (
            "addChatMember",
            "adds a new member to a chat; requires can_invite_users member right. members can't be added to private or secret chats. returns information about members that weren't added",
        ) => Some(Contract::MemberRightInKinds {
            right: ChatMemberRight::CanInviteUsers,
            kinds: &[
                ResolvedChatKind::BasicGroup,
                ResolvedChatKind::Supergroup,
                ResolvedChatKind::Channel,
            ],
        }),
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
            "unpinChatMessage",
            "removes a pinned message from a chat; requires can_pin_messages member right if the chat is a basic group or supergroup, or can_edit_messages administrator right if the chat is a channel",
        ) => Some(Contract::ConditionalUnpin),
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
    runtime_signal_dispositions_with_consumed(method, &consumed)
}

fn runtime_signal_dispositions_with_consumed(
    method: &Definition,
    consumed: &BTreeSet<RuntimeSignalKey>,
) -> Result<Vec<(RuntimeSignalKey, RuntimeSignalDisposition)>, CapabilityGenerationError> {
    documented_runtime_signal_keys(method).map(|keys| {
        keys.into_iter()
            .map(|key| {
                let disposition = if is_terminal_non_gate(method, &key) {
                    RuntimeSignalDisposition::NotRuntimeGate(NonGateReason::ChatBoostVocabulary)
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

fn is_terminal_non_gate(method: &Definition, key: &RuntimeSignalKey) -> bool {
    let source_text = normalized_signal_source_text(method, key.source());
    match (method.name(), key.source(), key.family()) {
        (
            "getChatBoostFeatures",
            RuntimeSignalSource::Description,
            RuntimeSignalFamily::BoostLevelPhrase,
        ) => {
            source_text.as_deref()
                == Some(
                    "returns the list of features available for different chat boost levels. this is an offline method",
                )
        }
        (
            "getChatBoostLevelFeatures",
            RuntimeSignalSource::Description,
            RuntimeSignalFamily::BoostLevelPhrase,
        ) => {
            source_text.as_deref()
                == Some(
                    "returns the list of features available on the specific chat boost level. this is an offline method",
                )
        }
        (
            "getChatBoostLevelFeatures",
            RuntimeSignalSource::Argument(argument),
            RuntimeSignalFamily::BoostLevelPhrase,
        ) => argument.as_str() == "level" && source_text.as_deref() == Some("chat boost level"),
        _ => false,
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
        ("engine.rs", include_bytes!("engine.rs").as_slice()),
        (
            "telegram-core/method_capability.rs",
            include_bytes!("../../../crates/telegram-core/src/method_capability.rs").as_slice(),
        ),
        (
            "telegram-core/feature.rs",
            include_bytes!("../../../crates/telegram-core/src/feature.rs").as_slice(),
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
struct OwnerManifest {
    mapping_sha256: String,
    methods: Vec<OwnerMethod>,
}

#[derive(Debug, Deserialize)]
struct OwnerMethod {
    method: String,
    signature_sha256: String,
    feature_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CapabilityPolicyDto {
    format_version: u32,
    schema_sha256: String,
    owner_mapping_sha256: String,
    methods: Vec<MethodPolicyDto>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MethodPolicyDto {
    method: String,
    signature_sha256: String,
    documentation_sha256: String,
    feature_id: String,
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
    owner_mapping_sha256: &'a str,
    methods: &'a [CanonicalMethodRow],
}

#[derive(Serialize)]
struct GeneratedManifest {
    format_version: u32,
    generated_by: &'static str,
    engine_source_sha256: String,
    vendor_manifest_sha256: String,
    schema: SchemaEvidence,
    owner: OwnerEvidence,
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
struct OwnerEvidence {
    mapping_sha256: String,
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
    feature_id: &'static str,
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
        feature_id: &'static str,
        descriptor: CapabilityDescriptor,
        rationale: String,
    ) -> Self {
        Self {
            method,
            signature_sha256,
            documentation_sha256,
            feature_id,
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
}

impl CanonicalRuntimeRequirement {
    fn from_domain(value: &RuntimeRequirement) -> Self {
        match value {
            RuntimeRequirement::ChatKind(condition) => Self::ChatKind {
                target: CanonicalChatTarget::from_domain(condition.target()),
                value: condition.kind().as_str(),
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
    OwnerGeneration,
    InvalidSchema,
    SchemaDrift,
    OwnerDrift,
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
