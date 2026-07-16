//! Discovery и universal schema-validated call поверх generated registry.

use serde::Serialize;
use serde_json::Value;
use std::error::Error;
use std::fmt;
use std::num::NonZeroUsize;
use std::time::Duration;
use std::time::{Instant, SystemTime};

use crate::approval::{approval_required, ApprovedPlan, PlanAuthorizationError};
use crate::registry::{
    self, AccountKind, CapabilityDescriptor, CapabilityDisposition, RiskClass, SymbolDescriptor,
    TdObject, ValidatedRequest, ValidationError, BUILTINS, CAPABILITIES, CONSTRUCTORS, TYPES,
};
use crate::retry::{self, AttemptFailure, RetryExecution};
use crate::runtime::CoreRuntime;
use crate::transport::TransportError;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct VersionInfo<'runtime> {
    pub tdlib_version: &'runtime str,
    pub tdlib_commit: &'runtime str,
    pub schema_sha256: &'static str,
}

pub fn version(runtime: &CoreRuntime) -> VersionInfo<'_> {
    VersionInfo {
        tdlib_version: runtime.identity().version(),
        tdlib_commit: runtime.identity().commit(),
        schema_sha256: registry::SCHEMA.sha256,
    }
}

pub fn capabilities() -> &'static [CapabilityDescriptor] {
    CAPABILITIES
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemaSearchResult {
    Symbol(&'static SymbolDescriptor),
    Type(&'static str),
}

impl SchemaSearchResult {
    pub fn name(self) -> &'static str {
        match self {
            Self::Symbol(symbol) => symbol.name,
            Self::Type(name) => name,
        }
    }
}

pub fn schema_search(query: &str) -> Vec<SchemaSearchResult> {
    let query = query.to_ascii_lowercase();
    let terms = query.split_whitespace().collect::<Vec<_>>();
    let mut results = BUILTINS
        .iter()
        .chain(CONSTRUCTORS)
        .chain(registry::METHODS)
        .filter(|symbol| {
            let name = symbol.name.to_ascii_lowercase();
            let signature = symbol.signature.to_ascii_lowercase();
            let documentation = symbol.documentation.to_ascii_lowercase();
            terms.iter().all(|term| {
                name.contains(term) || signature.contains(term) || documentation.contains(term)
            })
        })
        .map(SchemaSearchResult::Symbol)
        .chain(
            TYPES
                .iter()
                .filter(|name| {
                    let name = name.to_ascii_lowercase();
                    terms.iter().all(|term| name.contains(term))
                })
                .map(|name| SchemaSearchResult::Type(name)),
        )
        .collect::<Vec<_>>();
    results.sort_unstable_by_key(|result| result.name());
    results
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemaDescription {
    Symbol(&'static SymbolDescriptor),
    Type {
        name: &'static str,
        constructors: Vec<&'static SymbolDescriptor>,
    },
}

pub fn schema_describe(name: &str) -> Option<SchemaDescription> {
    registry::method(name)
        .or_else(|| registry::constructor(name))
        .or_else(|| BUILTINS.iter().find(|symbol| symbol.name == name))
        .map(SchemaDescription::Symbol)
        .or_else(|| {
            TYPES
                .binary_search(&name)
                .ok()
                .map(|index| SchemaDescription::Type {
                    name: TYPES[index],
                    constructors: CONSTRUCTORS
                        .iter()
                        .filter(|constructor| constructor.result.name == name)
                        .collect(),
                })
        })
}

pub fn td_call(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    request: Value,
    deadline: Instant,
) -> Result<TdObject, RawApiError> {
    td_call_report(runtime, policy, request, deadline, &mut Some).map(|(response, _, _)| response)
}

pub fn td_call_observed(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    request: Value,
    deadline: Instant,
    mut on_flood: impl FnMut(Duration) -> Option<Duration>,
) -> Result<(TdObject, u64), RawApiError> {
    td_call_report(runtime, policy, request, deadline, &mut on_flood)
        .map(|(response, _, attempts)| (response, attempts.saturating_sub(1)))
}

pub(crate) fn td_call_with_boundary(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    request: Value,
    deadline: Instant,
) -> Result<(TdObject, u64), RawApiError> {
    td_call_report(runtime, policy, request, deadline, &mut Some)
        .map(|(response, boundary, _)| (response, boundary))
}

fn td_call_report(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    request: Value,
    deadline: Instant,
    on_flood: &mut impl FnMut(Duration) -> Option<Duration>,
) -> Result<(TdObject, u64, u64), RawApiError> {
    let request = ValidatedRequest::from_value(request).map_err(RawApiError::Validation)?;
    let method = request.descriptor();
    policy
        .authorize_request(&request)
        .map_err(RawApiError::Policy)?;
    let retry = registry::capability(method.name)
        .and_then(|capability| match capability.disposition {
            CapabilityDisposition::Reviewed { retry, .. } => Some(retry),
            CapabilityDisposition::DefaultDeny => None,
        })
        .expect("authorized requests have a reviewed retry class");
    if retry != crate::registry::RetryClass::SafeRead {
        return td_call_once(runtime, method, request.as_value(), deadline)
            .map(|(response, boundary)| (response, boundary, 1));
    }
    match retry::safe_read(
        method.name,
        request.as_value(),
        NonZeroUsize::new(2).expect("two is non-zero"),
        deadline,
        |request| match td_call_once(runtime, method, request, deadline) {
            Ok(result) => match flood_delay(result.0.as_value()) {
                Some(retry_after) => match on_flood(retry_after) {
                    Some(retry_after) => Err(AttemptFailure::Retryable {
                        retry_after,
                        error: ReadFailure::Flood(result),
                    }),
                    None => Err(AttemptFailure::Terminal(ReadFailure::Flood(result))),
                },
                None => Ok(result),
            },
            Err(error) => Err(AttemptFailure::Terminal(ReadFailure::Error(error))),
        },
    )
    .expect("generated retry class was checked")
    {
        RetryExecution::Succeeded {
            value: (response, boundary),
            attempts,
            ..
        } => Ok((response, boundary, attempts as u64)),
        RetryExecution::Stopped {
            error, attempts, ..
        } => match error {
            ReadFailure::Flood((response, boundary)) => Ok((response, boundary, attempts as u64)),
            ReadFailure::Error(error) => Err(error),
        },
    }
}

fn td_call_once(
    runtime: &CoreRuntime,
    method: &'static SymbolDescriptor,
    request: &Value,
    deadline: Instant,
) -> Result<(TdObject, u64), RawApiError> {
    let pending = runtime
        .transport()
        .request(request.clone())
        .map_err(RawApiError::Transport)?;
    let boundary = pending.correlation_id();
    let response = pending
        .wait_until(deadline)
        .map_err(RawApiError::Transport)?;
    let response = TdObject::from_value(response).map_err(RawApiError::Validation)?;
    if response.descriptor().is_some_and(|actual| {
        actual.name != "error"
            && method.result.name != "Object"
            && actual.name != method.result.name
            && actual.result.name != method.result.name
    }) {
        return Err(RawApiError::UnexpectedResult {
            method: method.name,
            expected: method.result.name,
        });
    }
    Ok((response, boundary))
}

enum ReadFailure {
    Flood((TdObject, u64)),
    Error(RawApiError),
}

fn flood_delay(response: &Value) -> Option<Duration> {
    if response.get("@type").and_then(Value::as_str) != Some("error")
        || response.get("code").and_then(Value::as_i64) != Some(429)
    {
        return None;
    }
    let seconds = response
        .get("message")
        .and_then(Value::as_str)?
        .rsplit_once("retry after ")?
        .1
        .parse()
        .ok()?;
    Some(Duration::from_secs(seconds))
}

#[derive(Debug)]
pub struct RawPolicy {
    account: AccountKind,
    allowed_risks: Vec<RiskClass>,
    approval: Option<ApprovedPlan>,
}

impl RawPolicy {
    pub fn new(account: AccountKind, allowed_risks: Vec<RiskClass>) -> Self {
        Self {
            account,
            allowed_risks,
            approval: None,
        }
    }

    pub fn with_approval(mut self, approval: ApprovedPlan) -> Self {
        self.approval = Some(approval);
        self
    }

    pub fn authorize(&self, method: &str) -> Result<(), PolicyError> {
        self.authorized_risk(method).map(|_| ())
    }

    pub(crate) fn authorize_request(&self, request: &ValidatedRequest) -> Result<(), PolicyError> {
        let risk = self.authorized_risk(request.descriptor().name)?;
        if approval_required(risk) {
            let approval = self
                .approval
                .as_ref()
                .ok_or(PolicyError::ApprovalRequired { risk })?;
            approval
                .authorize(request, SystemTime::now())
                .map_err(|reason| PolicyError::ApprovalDenied { reason })?;
        }
        Ok(())
    }

    fn authorized_risk(&self, method: &str) -> Result<RiskClass, PolicyError> {
        let capability = registry::capability(method).ok_or(PolicyError::DefaultDeny)?;
        let CapabilityDisposition::Reviewed { risk, accounts, .. } = capability.disposition else {
            return Err(PolicyError::DefaultDeny);
        };
        if !accounts.contains(&self.account) {
            return Err(PolicyError::AccountScopeDenied);
        }
        if !self.allowed_risks.contains(&risk) {
            return Err(PolicyError::RiskDenied { risk });
        }
        Ok(risk)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyError {
    DefaultDeny,
    AccountScopeDenied,
    RiskDenied { risk: RiskClass },
    ApprovalRequired { risk: RiskClass },
    ApprovalDenied { reason: PlanAuthorizationError },
}

impl fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DefaultDeny => formatter.write_str("TDLib method is not reviewed"),
            Self::AccountScopeDenied => {
                formatter.write_str("TDLib method is unavailable for this account kind")
            }
            Self::RiskDenied { .. } => {
                formatter.write_str("TDLib method risk is not granted by policy")
            }
            Self::ApprovalRequired { .. } => {
                formatter.write_str("TDLib method requires external plan approval")
            }
            Self::ApprovalDenied { .. } => {
                formatter.write_str("TDLib external plan approval is invalid")
            }
        }
    }
}

impl Error for PolicyError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RawApiError {
    Validation(ValidationError),
    Policy(PolicyError),
    Transport(TransportError),
    UnexpectedResult {
        method: &'static str,
        expected: &'static str,
    },
}

impl fmt::Display for RawApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(error) => write!(formatter, "TDJSON validation failed: {error}"),
            Self::Policy(error) => write!(formatter, "TDJSON policy denied call: {error}"),
            Self::Transport(error) => write!(formatter, "TDJSON transport failed: {error}"),
            Self::UnexpectedResult { method, expected } => {
                write!(
                    formatter,
                    "TDJSON `{method}` returned a value outside `{expected}`"
                )
            }
        }
    }
}

impl Error for RawApiError {}

#[cfg(test)]
mod tests;
