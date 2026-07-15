//! In-memory leases одной daemon-owned TDLib session.

use std::collections::{BTreeSet, HashMap};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use telegram_protocol::{LeaseErrorCode, LeaseId, LeaseView};

const MAX_LEASE_TTL_MS: u64 = 60_000;

struct LeaseRecord {
    principal: String,
    scopes: BTreeSet<String>,
    ttl_ms: u64,
    expires_at: Instant,
}

pub struct LeaseManager {
    epoch: u128,
    next_id: u64,
    leases: HashMap<LeaseId, LeaseRecord>,
}

impl Default for LeaseManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LeaseManager {
    pub fn new() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let epoch = nanos ^ ((std::process::id() as u128) << 64);
        Self::with_epoch(epoch)
    }

    fn with_epoch(epoch: u128) -> Self {
        Self {
            epoch,
            next_id: 1,
            leases: HashMap::new(),
        }
    }

    pub fn acquire(
        &mut self,
        principal: String,
        scopes: Vec<String>,
        ttl_ms: u64,
        now: Instant,
    ) -> Result<LeaseView, LeaseErrorCode> {
        validate_label(&principal).map_err(|_| LeaseErrorCode::InvalidPrincipal)?;
        let scopes = scopes
            .into_iter()
            .map(|scope| {
                validate_label(&scope)
                    .map(|_| scope)
                    .map_err(|_| LeaseErrorCode::InvalidScope)
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        if scopes.is_empty() {
            return Err(LeaseErrorCode::InvalidScope);
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
        Ok(())
    }

    pub fn expire(&mut self, now: Instant) -> usize {
        let before = self.leases.len();
        self.leases.retain(|_, lease| lease.expires_at > now);
        before - self.leases.len()
    }

    pub fn active_count(&self) -> usize {
        self.leases.len()
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
        let mut manager = LeaseManager::with_epoch(7);
        let lease = manager
            .acquire(
                "agent-a".to_owned(),
                vec!["write".to_owned(), "read".to_owned(), "read".to_owned()],
                1_000,
                start,
            )
            .unwrap();
        assert_eq!(lease.scopes, ["read", "write"]);
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
        let mut manager = LeaseManager::with_epoch(9);
        assert_eq!(
            manager.acquire("".to_owned(), vec!["read".to_owned()], 10, start),
            Err(LeaseErrorCode::InvalidPrincipal)
        );
        assert_eq!(
            manager.acquire(
                "agent".to_owned(),
                vec!["read".to_owned()],
                MAX_LEASE_TTL_MS + 1,
                start
            ),
            Err(LeaseErrorCode::InvalidTtl)
        );
        let lease = manager
            .acquire("agent".to_owned(), vec!["read".to_owned()], 10, start)
            .unwrap();
        assert_eq!(
            manager.heartbeat(&lease.lease_id, "agent", start + Duration::from_millis(10)),
            Err(LeaseErrorCode::LeaseExpired)
        );
        assert_eq!(manager.active_count(), 0);
    }
}
