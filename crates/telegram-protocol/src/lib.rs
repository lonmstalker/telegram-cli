//! Стабильные wire-контракты клиентов и daemon.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LeaseId(String);

impl LeaseId {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum LeaseRequest {
    LeaseAcquire {
        principal: String,
        scopes: Vec<String>,
        ttl_ms: u64,
    },
    LeaseHeartbeat {
        lease_id: LeaseId,
        principal: String,
    },
    LeaseRelease {
        lease_id: LeaseId,
        principal: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseView {
    pub lease_id: LeaseId,
    pub principal: String,
    pub scopes: Vec<String>,
    pub ttl_ms: u64,
    pub expires_in_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum LeaseResponse {
    LeaseGranted { lease: LeaseView },
    LeaseRenewed { lease: LeaseView },
    LeaseReleased { lease_id: LeaseId },
    Error { code: LeaseErrorCode },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseErrorCode {
    InvalidRequest,
    InvalidPrincipal,
    InvalidScope,
    InvalidTtl,
    LeaseNotFound,
    LeaseExpired,
    PrincipalMismatch,
    IdentifierExhausted,
}
