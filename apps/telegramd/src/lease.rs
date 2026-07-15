//! In-memory leases одной daemon-owned TDLib session.

use std::collections::{BTreeSet, HashMap};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use telegram_core::raw_api::RawPolicy;
use telegram_core::registry::{AccountKind, RiskClass};
use telegram_protocol::{LeaseErrorCode, LeaseId, LeaseView, RiskScope};

use crate::telemetry::Telemetry;

const MAX_LEASE_TTL_MS: u64 = 60_000;

struct LeaseRecord {
    principal: String,
    scopes: BTreeSet<RiskScope>,
    ttl_ms: u64,
    expires_at: Instant,
}

pub struct LeaseManager {
    epoch: u128,
    next_id: u64,
    leases: HashMap<LeaseId, LeaseRecord>,
    allowed_scopes: BTreeSet<RiskScope>,
    telemetry: Telemetry,
}

impl Default for LeaseManager {
    fn default() -> Self {
        Self::new([RiskScope::Read])
    }
}

impl LeaseManager {
    pub fn new(allowed_scopes: impl IntoIterator<Item = RiskScope>) -> Self {
        Self::with_telemetry(allowed_scopes, Telemetry::default())
    }

    pub fn with_telemetry(
        allowed_scopes: impl IntoIterator<Item = RiskScope>,
        telemetry: Telemetry,
    ) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let epoch = nanos ^ ((std::process::id() as u128) << 64);
        Self::with_epoch_and_telemetry(epoch, allowed_scopes, telemetry)
    }

    #[cfg(test)]
    fn with_epoch(epoch: u128, allowed_scopes: impl IntoIterator<Item = RiskScope>) -> Self {
        Self::with_epoch_and_telemetry(epoch, allowed_scopes, Telemetry::default())
    }

    fn with_epoch_and_telemetry(
        epoch: u128,
        allowed_scopes: impl IntoIterator<Item = RiskScope>,
        telemetry: Telemetry,
    ) -> Self {
        Self {
            epoch,
            next_id: 1,
            leases: HashMap::new(),
            allowed_scopes: allowed_scopes.into_iter().collect(),
            telemetry,
        }
    }

    pub fn acquire(
        &mut self,
        principal: String,
        scopes: Vec<RiskScope>,
        ttl_ms: u64,
        now: Instant,
    ) -> Result<LeaseView, LeaseErrorCode> {
        validate_label(&principal).map_err(|_| LeaseErrorCode::InvalidPrincipal)?;
        let scopes = scopes.into_iter().collect::<BTreeSet<_>>();
        if scopes.is_empty() {
            return Err(LeaseErrorCode::InvalidScope);
        }
        if !scopes.is_subset(&self.allowed_scopes) {
            return Err(LeaseErrorCode::ScopeDenied);
        }
        let ttl = Duration::from_millis(ttl_ms);
        let Some(expires_at) = (ttl_ms != 0 && ttl_ms <= MAX_LEASE_TTL_MS)
            .then(|| now.checked_add(ttl))
            .flatten()
        else {
            return Err(LeaseErrorCode::InvalidTtl);
        };
        self.expire(now);
        let id = self.next_lease_id()?;
        let record = LeaseRecord {
            principal,
            scopes,
            ttl_ms,
            expires_at,
        };
        let view = snapshot(&id, &record, now);
        self.leases.insert(id, record);
        self.telemetry.observe_leases(self.leases.len());
        Ok(view)
    }

    pub fn heartbeat(
        &mut self,
        lease_id: &LeaseId,
        principal: &str,
        now: Instant,
    ) -> Result<LeaseView, LeaseErrorCode> {
        self.reject_expired(lease_id, now)?;
        let record = self
            .leases
            .get_mut(lease_id)
            .ok_or(LeaseErrorCode::LeaseNotFound)?;
        if record.principal != principal {
            return Err(LeaseErrorCode::PrincipalMismatch);
        }
        record.expires_at = now
            .checked_add(Duration::from_millis(record.ttl_ms))
            .ok_or(LeaseErrorCode::InvalidTtl)?;
        Ok(snapshot(lease_id, record, now))
    }

    pub fn release(
        &mut self,
        lease_id: &LeaseId,
        principal: &str,
        now: Instant,
    ) -> Result<(), LeaseErrorCode> {
        self.reject_expired(lease_id, now)?;
        let record = self
            .leases
            .get(lease_id)
            .ok_or(LeaseErrorCode::LeaseNotFound)?;
        if record.principal != principal {
            return Err(LeaseErrorCode::PrincipalMismatch);
        }
        self.leases.remove(lease_id);
        self.telemetry.observe_leases(self.leases.len());
        Ok(())
    }

    pub fn expire(&mut self, now: Instant) -> usize {
        let before = self.leases.len();
        self.leases.retain(|_, lease| lease.expires_at > now);
        self.telemetry.observe_leases(self.leases.len());
        before - self.leases.len()
    }

    pub fn active_count(&self) -> usize {
        self.leases.len()
    }

    pub fn raw_policy(
        &mut self,
        lease_id: &LeaseId,
        principal: &str,
        account: AccountKind,
        now: Instant,
    ) -> Result<RawPolicy, LeaseErrorCode> {
        self.reject_expired(lease_id, now)?;
        let record = self
            .leases
            .get(lease_id)
            .ok_or(LeaseErrorCode::LeaseNotFound)?;
        if record.principal != principal {
            return Err(LeaseErrorCode::PrincipalMismatch);
        }
        Ok(RawPolicy::new(
            account,
            record.scopes.iter().copied().map(risk_class).collect(),
        ))
    }

    fn reject_expired(&mut self, lease_id: &LeaseId, now: Instant) -> Result<(), LeaseErrorCode> {
        if self
            .leases
            .get(lease_id)
            .is_some_and(|record| record.expires_at <= now)
        {
            self.leases.remove(lease_id);
            self.expire(now);
            return Err(LeaseErrorCode::LeaseExpired);
        }
        self.expire(now);
        Ok(())
    }

    fn next_lease_id(&mut self) -> Result<LeaseId, LeaseErrorCode> {
        let current = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or(LeaseErrorCode::IdentifierExhausted)?;
        Ok(LeaseId::new(format!("{:032x}-{current:016x}", self.epoch)))
    }
}

fn validate_label(value: &str) -> Result<(), ()> {
    if value.is_empty() || value.chars().any(char::is_control) {
        Err(())
    } else {
        Ok(())
    }
}

fn risk_class(scope: RiskScope) -> RiskClass {
    match scope {
        RiskScope::Read => RiskClass::Read,
        RiskScope::Presence => RiskClass::Presence,
        RiskScope::Send => RiskClass::Send,
        RiskScope::ReversibleMutation => RiskClass::ReversibleMutation,
        RiskScope::Admin => RiskClass::Admin,
        RiskScope::Destructive => RiskClass::Destructive,
        RiskScope::Financial => RiskClass::Financial,
        RiskScope::AuthSecurity => RiskClass::AuthSecurity,
    }
}

fn snapshot(id: &LeaseId, record: &LeaseRecord, now: Instant) -> LeaseView {
    LeaseView {
        lease_id: id.clone(),
        principal: record.principal.clone(),
        scopes: record.scopes.iter().cloned().collect(),
        ttl_ms: record.ttl_ms,
        expires_in_ms: record
            .expires_at
            .saturating_duration_since(now)
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heartbeat_extends_ttl_and_release_checks_principal() {
        let start = Instant::now();
        let mut manager = LeaseManager::with_epoch(7, [RiskScope::Read, RiskScope::Send]);
        let lease = manager
            .acquire(
                "agent-a".to_owned(),
                vec![RiskScope::Send, RiskScope::Read, RiskScope::Read],
                1_000,
                start,
            )
            .unwrap();
        assert_eq!(lease.scopes, [RiskScope::Read, RiskScope::Send]);
        assert_eq!(manager.active_count(), 1);
        assert_eq!(
            manager.release(&lease.lease_id, "agent-b", start),
            Err(LeaseErrorCode::PrincipalMismatch)
        );
        let renewed = manager
            .heartbeat(
                &lease.lease_id,
                "agent-a",
                start + Duration::from_millis(900),
            )
            .unwrap();
        assert_eq!(renewed.expires_in_ms, 1_000);
        manager
            .release(
                &lease.lease_id,
                "agent-a",
                start + Duration::from_millis(901),
            )
            .unwrap();
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn expired_or_invalid_lease_fails_closed() {
        let start = Instant::now();
        let mut manager = LeaseManager::with_epoch(9, [RiskScope::Read]);
        assert_eq!(
            manager.acquire("".to_owned(), vec![RiskScope::Read], 10, start),
            Err(LeaseErrorCode::InvalidPrincipal)
        );
        assert_eq!(
            manager.acquire(
                "agent".to_owned(),
                vec![RiskScope::Read],
                MAX_LEASE_TTL_MS + 1,
                start
            ),
            Err(LeaseErrorCode::InvalidTtl)
        );
        let lease = manager
            .acquire("agent".to_owned(), vec![RiskScope::Read], 10, start)
            .unwrap();
        assert_eq!(
            manager.heartbeat(&lease.lease_id, "agent", start + Duration::from_millis(10)),
            Err(LeaseErrorCode::LeaseExpired)
        );
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn owner_ceiling_and_lease_scopes_build_raw_policy() {
        let start = Instant::now();
        let mut manager = LeaseManager::with_epoch(11, [RiskScope::Read, RiskScope::Send]);
        assert_eq!(
            manager.acquire("agent".to_owned(), vec![RiskScope::Financial], 1_000, start,),
            Err(LeaseErrorCode::ScopeDenied)
        );

        let lease = manager
            .acquire("agent".to_owned(), vec![RiskScope::Read], 1_000, start)
            .unwrap();
        let policy = manager
            .raw_policy(&lease.lease_id, "agent", AccountKind::RegularUser, start)
            .unwrap();
        assert_eq!(policy.authorize("getChatStatistics"), Ok(()));
        assert_eq!(
            policy.authorize("sendBotStartMessage"),
            Err(telegram_core::raw_api::PolicyError::RiskDenied {
                risk: RiskClass::Send,
            })
        );
    }
}
