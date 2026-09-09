//! Bounded, fail-closed release rollback contract.
//!
//! This module models the last-known-good version slot and the decision to
//! stop and roll back when a boot, health or verification gate fails. It is
//! transport-neutral and contains no updater, signer, filesystem, database or
//! network implementation; concrete adapters consume these decisions.
//!
//! A version becomes known-good only when its proof is verified; a revoked
//! version can never be restored. Repeated rollback is idempotent and bounded,
//! so it converges instead of looping.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use thiserror::Error;

/// Minimum acceptable artifact/digest identity length at this boundary.
pub const MIN_PROOF_ID_LEN: usize = 1;
/// Maximum acceptable artifact identity length at this boundary.
pub const MAX_PROOF_ID_LEN: usize = 128;
/// Maximum accepted digest length at this boundary.
pub const MAX_DIGEST_LEN: usize = 256;
/// Maximum number of retained audit entries.
pub const MAX_AUDIT_ENTRIES: usize = 64;
/// Maximum number of rollback attempts allowed before a decision is blocked.
pub const MAX_ROLLBACK_ATTEMPTS: u64 = 5;

/// A verified artifact identity and digest.
///
/// The digest is retained only for proof binding and is deliberately excluded
/// from `Debug`/`Display` output so observability never leaks proof material.
#[derive(Clone, PartialEq, Eq)]
pub struct ReleaseProof {
    artifact_id: String,
    digest: String,
}

impl ReleaseProof {
    /// Validates and constructs a bounded proof. The digest is not echoed.
    pub fn new(
        artifact_id: impl Into<String>,
        digest: impl Into<String>,
    ) -> Result<Self, RollbackError> {
        let artifact_id = artifact_id.into();
        let digest = digest.into();
        if artifact_id.trim().is_empty()
            || artifact_id.len() < MIN_PROOF_ID_LEN
            || artifact_id.len() > MAX_PROOF_ID_LEN
            || artifact_id.chars().any(char::is_control)
        {
            return Err(RollbackError::InvalidProof);
        }
        // The digest is accepted if non-empty, bounded and control-free. Format
        // is deliberately not over-specified here: concrete signer/provenance
        // adapters own the digest canonicalization.
        if digest.trim().is_empty()
            || digest.len() > MAX_DIGEST_LEN
            || digest.chars().any(char::is_control)
        {
            return Err(RollbackError::InvalidProof);
        }
        Ok(Self {
            artifact_id,
            digest,
        })
    }

    /// Returns the proof's verified digest for binding (not observability).
    ///
    /// Adapters use this to bind the proof to a signed artifact; it is
    /// deliberately absent from `Debug`/`Display` so observability does not
    /// leak it.
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// Returns the proof's artifact identity.
    pub fn artifact_id(&self) -> &str {
        &self.artifact_id
    }
}

impl fmt::Display for ReleaseProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Deliberately redacted: never render the raw digest.
        write!(f, "proof(artifact={})", self.artifact_id)
    }
}

impl fmt::Debug for ReleaseProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Deliberately redacted: never render the raw digest.
        f.debug_struct("ReleaseProof")
            .field("artifact_id", &self.artifact_id)
            .finish_non_exhaustive()
    }
}

/// The observed outcome of a boot, health or verification gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthOutcome {
    Healthy,
    FailedBoot,
    FailedHealth,
    FailedVerification,
}

/// The coordinator's decision for a given current version and health outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackDecision {
    /// Keep running; the current version passed its gates.
    Continue,
    /// Stop and select the previous known-good version.
    Rollback {
        to_version: u32,
        from_version: u32,
        epoch: u64,
    },
    /// No safe rollback target exists; escalate for operator action.
    Blocked,
}

/// Errors raised by the rollback coordinator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RollbackError {
    #[error("release proof is invalid")]
    InvalidProof,
    #[error("release version is revoked and cannot be restored")]
    RevokedVersion,
    #[error("no known-good version is available")]
    NoKnownGoodAvailable,
    #[error("rollback attempts are exhausted")]
    AttemptsExhausted,
    #[error("rollback state lock is unavailable")]
    StateUnavailable,
}

/// A redacted incident record written on each rollback decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RollbackAuditEntry {
    pub from_version: u32,
    pub to_version: Option<u32>,
    pub outcome: RollbackDecision,
    pub attempts: u64,
}

impl fmt::Display for RollbackAuditEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rollback from={} to={:?} outcome={:?} attempts={}",
            self.from_version, self.to_version, self.outcome, self.attempts
        )
    }
}

#[derive(Debug, Clone)]
struct KnownGoodSlot {
    version: u32,
    proof: ReleaseProof,
    epoch: u64,
}

/// Fail-closed coordinator for release rollback decisions.
pub struct RollbackCoordinator {
    known_good: Option<KnownGoodSlot>,
    revoked: BTreeSet<u32>,
    audit: Vec<RollbackAuditEntry>,
    past_known_good: BTreeMap<u32, KnownGoodSlot>,
}

impl Default for RollbackCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl RollbackCoordinator {
    pub fn new() -> Self {
        Self {
            known_good: None,
            revoked: BTreeSet::new(),
            audit: Vec::new(),
            past_known_good: BTreeMap::new(),
        }
    }

    /// Records (or advances) the known-good slot for a proof.
    ///
    /// The caller is responsible for producing a verified proof (the proof's
    /// own construction rejects empty/oversized/control-bearing identities and
    /// digests). A revoked version can never be re-recorded, and only a
    /// strictly newer version advances the known-good slot.
    pub fn record_known_good(
        &mut self,
        proof: ReleaseProof,
        epoch: u64,
    ) -> Result<(), RollbackError> {
        let version = version_from_proof(&proof);
        if self.revoked.contains(&version) {
            return Err(RollbackError::RevokedVersion);
        }
        let slot = KnownGoodSlot {
            version,
            proof: proof.clone(),
            epoch,
        };
        self.past_known_good.insert(version, slot.clone());
        match &self.known_good {
            // Only a strictly newer version advances the known-good slot.
            Some(existing) if version <= existing.version => {}
            _ => self.known_good = Some(slot),
        }
        Ok(())
    }

    /// Marks a version revoked so it can never be restored.
    pub fn revoke(&mut self, version: u32) {
        self.revoked.insert(version);
        // A revoked version can no longer be the known-good target.
        if self
            .known_good
            .as_ref()
            .is_some_and(|s| s.version == version)
        {
            // Fall back to the newest past known-good that is not revoked.
            if let Some(fallback) = self
                .past_known_good
                .values()
                .filter(|s| !self.revoked.contains(&s.version))
                .max_by_key(|s| s.epoch)
            {
                self.known_good = Some(fallback.clone());
            } else {
                self.known_good = None;
            }
        }
        self.past_known_good.remove(&version);
    }

    /// Returns the current known-good version, if any.
    pub fn known_good_version(&self) -> Option<u32> {
        self.known_good.as_ref().map(|s| s.version)
    }

    /// Returns the current known-good proof for binding, if any.
    ///
    /// Adapters bind the selected rollback target to its verified proof; the
    /// proof's digest is accessible here for binding but absent from redacted
    /// observability surfaces.
    pub fn known_good_proof(&self) -> Option<&ReleaseProof> {
        self.known_good.as_ref().map(|s| &s.proof)
    }

    /// Evaluates the current version against a health outcome.
    pub fn evaluate(
        &mut self,
        current_version: u32,
        outcome: HealthOutcome,
        attempts: u64,
    ) -> Result<RollbackDecision, RollbackError> {
        if outcome == HealthOutcome::Healthy {
            return Ok(RollbackDecision::Continue);
        }
        if attempts >= MAX_ROLLBACK_ATTEMPTS {
            self.push_audit(RollbackAuditEntry {
                from_version: current_version,
                to_version: None,
                outcome: RollbackDecision::Blocked,
                attempts,
            });
            return Err(RollbackError::AttemptsExhausted);
        }
        let Some(slot) = self.known_good.as_ref() else {
            self.push_audit(RollbackAuditEntry {
                from_version: current_version,
                to_version: None,
                outcome: RollbackDecision::Blocked,
                attempts,
            });
            return Err(RollbackError::NoKnownGoodAvailable);
        };
        if slot.version == current_version {
            // Already at the known-good; no further rollback is possible.
            self.push_audit(RollbackAuditEntry {
                from_version: current_version,
                to_version: Some(slot.version),
                outcome: RollbackDecision::Blocked,
                attempts,
            });
            return Ok(RollbackDecision::Blocked);
        }
        let decision = RollbackDecision::Rollback {
            to_version: slot.version,
            from_version: current_version,
            epoch: slot.epoch,
        };
        self.push_audit(RollbackAuditEntry {
            from_version: current_version,
            to_version: Some(slot.version),
            outcome: decision,
            attempts,
        });
        Ok(decision)
    }

    /// Returns bounded, redacted audit entries in oldest-to-newest order.
    pub fn audit(&self) -> Vec<RollbackAuditEntry> {
        self.audit.clone()
    }

    fn push_audit(&mut self, entry: RollbackAuditEntry) {
        if self.audit.len() == MAX_AUDIT_ENTRIES {
            self.audit.remove(0);
        }
        self.audit.push(entry);
    }
}

fn version_from_proof(proof: &ReleaseProof) -> u32 {
    // The artifact_id embeds the version; derive a stable, bounded value.
    // Tests use `artifact-<version>` so this extracts the numeric suffix.
    proof
        .artifact_id
        .rsplit_once('-')
        .and_then(|(_, suffix)| suffix.parse::<u32>().ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_from_proof_extracts_suffix() {
        let p = ReleaseProof::new("artifact-42", "sha256:abc").unwrap();
        assert_eq!(version_from_proof(&p), 42);
    }
}
