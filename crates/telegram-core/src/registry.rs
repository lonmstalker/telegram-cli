//! Generated descriptors и общий lossless TDJSON codec закреплённой схемы.

use base64::Engine as _;
use serde::Serialize;
use serde_json::{Map, Value};
use std::error::Error;
use std::fmt;

mod generated;

pub use generated::{
    AUTHORIZATION_STATES, BUILTINS, CAPABILITIES, CONSTRUCTORS, METHODS, SCHEMA, TYPES, UPDATES,
};

const TYPE_FIELD: &str = "@type";
const MAX_INT53: i64 = (1_i64 << 53) - 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct SchemaDescriptor {
    pub version: &'static str,
    pub commit: &'static str,
    pub sha256: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Builtin,
    Constructor,
    Method,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct TypeDescriptor {
    pub name: &'static str,
    pub arguments: &'static [TypeDescriptor],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct FieldDescriptor {
    pub name: &'static str,
    pub ty: TypeDescriptor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct SymbolDescriptor {
    pub kind: SymbolKind,
    pub name: &'static str,
    pub result: TypeDescriptor,
    pub signature: &'static str,
    pub documentation: &'static str,
    pub fields: &'static [FieldDescriptor],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    RegularUser,
    Bot,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskClass {
    Read,
    Presence,
    Send,
    ReversibleMutation,
    Admin,
    Destructive,
    Financial,
    AuthSecurity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryClass {
    SafeRead,
    Convergent,
    Reconcile,
    Never,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CapabilityDisposition {
    DefaultDeny,
    Reviewed {
        risk: RiskClass,
        accounts: &'static [AccountKind],
        runtime_requirements: &'static str,
        retry: RetryClass,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct CapabilityDescriptor {
    pub method: &'static str,
    pub disposition: CapabilityDisposition,
}

pub fn method(name: &str) -> Option<&'static SymbolDescriptor> {
    find(METHODS, name)
}

pub fn constructor(name: &str) -> Option<&'static SymbolDescriptor> {
    find(CONSTRUCTORS, name)
}

pub fn capability(name: &str) -> Option<&'static CapabilityDescriptor> {
    CAPABILITIES
        .binary_search_by_key(&name, |descriptor| descriptor.method)
        .ok()
        .map(|index| &CAPABILITIES[index])
}

fn find(symbols: &'static [SymbolDescriptor], name: &str) -> Option<&'static SymbolDescriptor> {
    symbols
        .binary_search_by_key(&name, |descriptor| descriptor.name)
        .ok()
        .map(|index| &symbols[index])
}

#[derive(Clone, Debug, PartialEq)]
pub struct TdObject(Value);

impl TdObject {
    pub fn from_value(value: Value) -> Result<Self, ValidationError> {
        type_name(&value)?;
        Ok(Self(value))
    }

    pub fn descriptor(&self) -> Option<&'static SymbolDescriptor> {
        type_name(&self.0).ok().and_then(constructor)
    }

    pub fn as_value(&self) -> &Value {
        &self.0
    }

    pub fn into_value(self) -> Value {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedRequest {
    method: &'static SymbolDescriptor,
    value: Value,
}

impl ValidatedRequest {
    pub fn from_value(value: Value) -> Result<Self, ValidationError> {
        let name = type_name(&value)?;
        let descriptor = method(name).ok_or(ValidationError::UnknownMethod)?;
        validate_fields(
            descriptor,
            value.as_object().ok_or(ValidationError::ExpectedObject)?,
        )?;
        Ok(Self {
            method: descriptor,
            value,
        })
    }

    pub fn descriptor(&self) -> &'static SymbolDescriptor {
        self.method
    }

    pub fn as_value(&self) -> &Value {
        &self.value
    }

    pub fn into_value(self) -> Value {
        self.value
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationError {
    ExpectedObject,
    MissingType,
    InvalidType,
    UnknownMethod,
    UnexpectedField {
        symbol: &'static str,
    },
    InvalidField {
        symbol: &'static str,
        field: &'static str,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpectedObject => formatter.write_str("TDJSON value must be an object"),
            Self::MissingType => formatter.write_str("TDJSON object is missing `@type`"),
            Self::InvalidType => formatter.write_str("TDJSON `@type` must be a non-empty string"),
            Self::UnknownMethod => {
                formatter.write_str("TDJSON method is absent from pinned schema")
            }
            Self::UnexpectedField { symbol } => {
                write!(formatter, "TDJSON `{symbol}` contains an unknown field")
            }
            Self::InvalidField { symbol, field } => {
                write!(formatter, "TDJSON `{symbol}.{field}` has an invalid value")
            }
        }
    }
}

impl Error for ValidationError {}

fn type_name(value: &Value) -> Result<&str, ValidationError> {
    let object = value.as_object().ok_or(ValidationError::ExpectedObject)?;
    match object.get(TYPE_FIELD) {
        None => Err(ValidationError::MissingType),
        Some(Value::String(name)) if !name.is_empty() => Ok(name),
        Some(_) => Err(ValidationError::InvalidType),
    }
}

fn validate_fields(
    descriptor: &'static SymbolDescriptor,
    object: &Map<String, Value>,
) -> Result<(), ValidationError> {
    if object
        .keys()
        .any(|name| name != TYPE_FIELD && !descriptor.fields.iter().any(|field| field.name == name))
    {
        return Err(ValidationError::UnexpectedField {
            symbol: descriptor.name,
        });
    }
    for field in descriptor.fields {
        if let Some(value) = object.get(field.name) {
            if !validate_value(field.ty, value) {
                return Err(ValidationError::InvalidField {
                    symbol: descriptor.name,
                    field: field.name,
                });
            }
        }
    }
    Ok(())
}

fn validate_value(ty: TypeDescriptor, value: &Value) -> bool {
    match ty.name {
        "double" => value.is_number(),
        "string" => value.is_string(),
        "bytes" => value.as_str().is_some_and(|text| {
            base64::engine::general_purpose::STANDARD
                .decode(text)
                .is_ok()
        }),
        "int32" => integer(value).is_some_and(|number| i32::try_from(number).is_ok()),
        "int53" => integer(value).is_some_and(|number| (-MAX_INT53..=MAX_INT53).contains(&number)),
        "int64" => integer(value).is_some(),
        "Bool" => value.is_boolean(),
        "vector" => {
            let Some(item_type) = ty.arguments.first().copied() else {
                return false;
            };
            value
                .as_array()
                .is_some_and(|items| items.iter().all(|item| validate_value(item_type, item)))
        }
        expected if value.is_null() => !matches!(
            expected,
            "double" | "string" | "bytes" | "int32" | "int53" | "int64" | "Bool" | "vector"
        ),
        expected => {
            let Ok(actual) = type_name(value) else {
                return false;
            };
            let Some(descriptor) = constructor(actual) else {
                return false;
            };
            if expected != "Object" && actual != expected && descriptor.result.name != expected {
                return false;
            }
            value
                .as_object()
                .is_some_and(|object| validate_fields(descriptor, object).is_ok())
        }
    }
}

fn integer(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
}

#[cfg(test)]
mod tests;
