//! Exact-plan preview и verification внешней one-shot approval capability.

use std::collections::HashSet;
use std::fmt;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use ed25519_dalek::{Signature, VerifyingKey};

use crate::idempotency::OperationFingerprint;
use crate::registry::{self, CapabilityDisposition, RetryClass, RiskClass, ValidatedRequest};

const DOMAIN: &[u8] = b"telegram-cli-plan-approval-v1";

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct PlanHash([u8; 32]);

impl PlanHash {
    pub fn as_bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn to_hex(self) -> String {
        OperationFingerprint::from_digest(self.0).to_hex()
    }
}

impl fmt::Debug for PlanHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("PlanHash")
            .field(&self.to_hex())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlanPreview {
    pub method: &'static str,
    pub risk: RiskClass,
    pub retry: RetryClass,
    pub hash: PlanHash,
}

impl PlanPreview {
    pub fn for_request(request: &ValidatedRequest) -> Result<Self, ApprovalError> {
        let capability =
            registry::capability(request.descriptor().name).ok_or(ApprovalError::DefaultDeny)?;
        let CapabilityDisposition::Reviewed { risk, retry, .. } = capability.disposition else {
            return Err(ApprovalError::DefaultDeny);
        };
        if !approval_required(risk) {
            return Err(ApprovalError::NotRequired);
        }
        Ok(Self {
            method: request.descriptor().name,
            risk,
            retry,
            hash: request_hash(request),
        })
    }
}

#[derive(Clone, Copy)]
pub struct ApprovalReceipt {
    plan_hash: PlanHash,
    expires_at_unix: u64,
    nonce: [u8; 16],
    signature: [u8; 64],
}

impl ApprovalReceipt {
    pub fn new(
        plan_hash: PlanHash,
        expires_at_unix: u64,
        nonce: [u8; 16],
        signature: [u8; 64],
    ) -> Self {
        Self {
            plan_hash,
            expires_at_unix,
            nonce,
            signature,
        }
    }
}

pub fn approval_payload(plan_hash: PlanHash, expires_at_unix: u64, nonce: [u8; 16]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(DOMAIN.len() + 32 + 8 + 16);
    payload.extend_from_slice(DOMAIN);
    payload.extend_from_slice(&plan_hash.0);
    payload.extend_from_slice(&expires_at_unix.to_be_bytes());
    payload.extend_from_slice(&nonce);
    payload
}

pub struct ApprovalVerifier {
    key: VerifyingKey,
    used_nonces: Mutex<HashSet<[u8; 16]>>,
}

impl ApprovalVerifier {
    pub fn from_hex(public_key: &str) -> Result<Self, ApprovalError> {
        if public_key.len() != 64 {
            return Err(ApprovalError::InvalidKey);
        }
        let mut bytes = [0; 32];
        for (target, pair) in bytes.iter_mut().zip(public_key.as_bytes().chunks_exact(2)) {
            *target = (hex_digit(pair[0]).ok_or(ApprovalError::InvalidKey)? << 4)
                | hex_digit(pair[1]).ok_or(ApprovalError::InvalidKey)?;
        }
        Self::new(bytes)
    }

    pub fn new(public_key: [u8; 32]) -> Result<Self, ApprovalError> {
        let key = VerifyingKey::from_bytes(&public_key).map_err(|_| ApprovalError::InvalidKey)?;
        Ok(Self {
            key,
            used_nonces: Mutex::new(HashSet::new()),
        })
    }

    pub fn verify(
        &self,
        preview: PlanPreview,
        receipt: ApprovalReceipt,
        now: SystemTime,
    ) -> Result<ApprovedPlan, ApprovalError> {
        if receipt.plan_hash != preview.hash {
            return Err(ApprovalError::HashMismatch);
        }
        let now = unix_time(now).map_err(|_| ApprovalError::InvalidClock)?;
        if receipt.expires_at_unix <= now {
            return Err(ApprovalError::Expired);
        }
        self.key
            .verify_strict(
                &approval_payload(preview.hash, receipt.expires_at_unix, receipt.nonce),
                &Signature::from_bytes(&receipt.signature),
            )
            .map_err(|_| ApprovalError::InvalidSignature)?;
        let mut used = self
            .used_nonces
            .lock()
            .map_err(|_| ApprovalError::Unavailable)?;
        if !used.insert(receipt.nonce) {
            return Err(ApprovalError::Replay);
        }
        Ok(ApprovedPlan {
            hash: preview.hash,
            expires_at_unix: receipt.expires_at_unix,
            consumed: AtomicBool::new(false),
        })
    }
}

pub struct ApprovedPlan {
    hash: PlanHash,
    expires_at_unix: u64,
    consumed: AtomicBool,
}

impl ApprovedPlan {
    pub(crate) fn authorize(
        &self,
        request: &ValidatedRequest,
        now: SystemTime,
    ) -> Result<(), PlanAuthorizationError> {
        if self.hash != request_hash(request) {
            return Err(PlanAuthorizationError::HashMismatch);
        }
        let now = unix_time(now).map_err(|_| PlanAuthorizationError::Expired)?;
        if self.expires_at_unix <= now {
            return Err(PlanAuthorizationError::Expired);
        }
        if self.consumed.swap(true, Ordering::AcqRel) {
            return Err(PlanAuthorizationError::Consumed);
        }
        Ok(())
    }
}

impl fmt::Debug for ApprovedPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApprovedPlan")
            .field("hash", &self.hash)
            .field("expires_at_unix", &self.expires_at_unix)
            .field("consumed", &self.consumed.load(Ordering::Acquire))
            .finish()
    }
}

pub(crate) fn approval_required(risk: RiskClass) -> bool {
    matches!(
        risk,
        RiskClass::Admin | RiskClass::Destructive | RiskClass::Financial | RiskClass::AuthSecurity
    )
}

fn request_hash(request: &ValidatedRequest) -> PlanHash {
    PlanHash(OperationFingerprint::for_request(request).as_bytes())
}

fn unix_time(time: SystemTime) -> Result<u64, ()> {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| ())
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanAuthorizationError {
    HashMismatch,
    Expired,
    Consumed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalError {
    DefaultDeny,
    NotRequired,
    InvalidKey,
    HashMismatch,
    InvalidClock,
    Expired,
    InvalidSignature,
    Replay,
    Unavailable,
}

impl fmt::Display for ApprovalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DefaultDeny => formatter.write_str("method is not reviewed"),
            Self::NotRequired => formatter.write_str("method doesn't require external approval"),
            Self::InvalidKey => formatter.write_str("approval public key is invalid"),
            Self::HashMismatch => formatter.write_str("approval plan hash doesn't match"),
            Self::InvalidClock => formatter.write_str("approval clock is invalid"),
            Self::Expired => formatter.write_str("approval has expired"),
            Self::InvalidSignature => formatter.write_str("approval signature is invalid"),
            Self::Replay => formatter.write_str("approval nonce was already used"),
            Self::Unavailable => formatter.write_str("approval verifier is unavailable"),
        }
    }
}

impl std::error::Error for ApprovalError {}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::json;

    use super::*;
    use crate::raw_api::{PolicyError, RawPolicy};
    use crate::registry::AccountKind;

    #[test]
    fn signed_exact_plan_is_one_shot_and_cannot_be_forged() {
        let operation = request(7);
        let preview = PlanPreview::for_request(&operation).unwrap();
        assert_eq!(preview.risk, RiskClass::Destructive);
        assert_ne!(
            preview.hash,
            PlanPreview::for_request(&request(8)).unwrap().hash
        );

        let signing = SigningKey::from_bytes(&[7; 32]);
        let verifier = ApprovalVerifier::new(signing.verifying_key().to_bytes()).unwrap();
        let now = SystemTime::now();
        let expires = unix_time(now.checked_add(Duration::from_secs(60)).unwrap()).unwrap();
        let nonce = [3; 16];
        let signature = signing
            .sign(&approval_payload(preview.hash, expires, nonce))
            .to_bytes();
        let receipt = ApprovalReceipt::new(preview.hash, expires, nonce, signature);
        let approved = verifier.verify(preview, receipt, now).unwrap();
        let policy = RawPolicy::new(AccountKind::RegularUser, vec![RiskClass::Destructive])
            .with_approval(approved);

        assert_eq!(policy.authorize_request(&operation), Ok(()));
        assert_eq!(
            policy.authorize_request(&operation),
            Err(PolicyError::ApprovalDenied {
                reason: PlanAuthorizationError::Consumed,
            })
        );
        assert!(matches!(
            verifier.verify(preview, receipt, now),
            Err(ApprovalError::Replay)
        ));

        let forged = ApprovalReceipt::new(preview.hash, expires, [4; 16], [0; 64]);
        assert!(matches!(
            verifier.verify(preview, forged, now),
            Err(ApprovalError::InvalidSignature)
        ));
    }

    #[test]
    fn high_risk_policy_requires_matching_approval() {
        let request = request(7);
        let policy = RawPolicy::new(AccountKind::RegularUser, vec![RiskClass::Destructive]);
        assert_eq!(
            policy.authorize_request(&request),
            Err(PolicyError::ApprovalRequired {
                risk: RiskClass::Destructive,
            })
        );
    }

    #[test]
    fn chat_folder_cleanup_plan_binds_target_and_leave_list() {
        let request = ValidatedRequest::from_value(json!({
            "@type": "deleteChatFolder",
            "chat_folder_id": 17,
            "leave_chat_ids": [],
        }))
        .unwrap();
        let with_leave = ValidatedRequest::from_value(json!({
            "@type": "deleteChatFolder",
            "chat_folder_id": 17,
            "leave_chat_ids": [23],
        }))
        .unwrap();

        let preview = PlanPreview::for_request(&request).unwrap();
        assert_eq!(preview.risk, RiskClass::Destructive);
        assert_eq!(preview.retry, RetryClass::Reconcile);
        assert_ne!(
            preview.hash,
            PlanPreview::for_request(&with_leave).unwrap().hash
        );
    }

    fn request(chat_id: i64) -> ValidatedRequest {
        ValidatedRequest::from_value(json!({
            "@type": "upgradeBasicGroupChatToSupergroupChat",
            "chat_id": chat_id,
        }))
        .unwrap()
    }
}
