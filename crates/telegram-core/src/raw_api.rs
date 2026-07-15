//! Discovery и universal schema-validated call поверх generated registry.

use serde_json::Value;
use std::error::Error;
use std::fmt;
use std::time::Instant;

use crate::registry::{
    self, CapabilityDescriptor, SymbolDescriptor, TdObject, ValidatedRequest, ValidationError,
    BUILTINS, CAPABILITIES, CONSTRUCTORS, TYPES,
};
use crate::runtime::CoreRuntime;
use crate::transport::TransportError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    request: Value,
    deadline: Instant,
) -> Result<TdObject, RawApiError> {
    let request = ValidatedRequest::from_value(request).map_err(RawApiError::Validation)?;
    let method = request.descriptor();
    let response = runtime
        .transport()
        .call_until(request.into_value(), deadline)
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
    Ok(response)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RawApiError {
    Validation(ValidationError),
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
