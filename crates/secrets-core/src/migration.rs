//! Privacy-preserving migration of legacy credential records.
//!
//! This module owns the orchestration boundary only. Legacy discovery, the
//! authenticated envelope codec, encrypted staging, the destination backend,
//! and the durable journal are injected ports. The coordinator never logs,
//! serializes, or stores secret material; material exists only in memory while
//! crossing a port and is zeroed when [`SecretMaterial`] is dropped.

use crate::{SecretMaterial, SecretStoreError, SecureSecretBackend, SecureSecretStore};
use provider_core::credentials::{CredentialAccessContext, CredentialAccount};
use provider_core::CredentialRef;
use std::fmt;
use std::sync::Arc;
use subtle::ConstantTimeEq;
use thiserror::Error;

pub const MAX_MIGRATION_ID_LEN: usize = 128;
pub const MAX_LEGACY_SOURCE_ID_LEN: usize = 128;
pub const MAX_POLICY_REVISION_LEN: usize = 64;
pub const MAX_STAGING_RECEIPT_LEN: usize = 128;
pub const MAX_STAGED_CIPHERTEXT_BYTES: usize = 68 * 1024;
pub const MAX_MIGRATION_LEASE_TOKEN_LEN: usize = 128;
pub const MIGRATION_LEASE_TTL_MS: u64 = 60_000;
const MAX_ACTOR_ID_LEN: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct SecretMigrationId(String);

impl SecretMigrationId {
    pub fn parse(value: impl Into<String>) -> Result<Self, SecretMigrationError> {
        let value = value.into();
        if !valid_prefixed_identifier(&value, "migration_", MAX_MIGRATION_ID_LEN) {
            return Err(SecretMigrationError::InvalidMetadata);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct LegacySourceId(String);

impl LegacySourceId {
    pub fn parse(value: impl Into<String>) -> Result<Self, SecretMigrationError> {
        let value = value.into();
        if !valid_prefixed_identifier(&value, "legacy_", MAX_LEGACY_SOURCE_ID_LEN) {
            return Err(SecretMigrationError::InvalidMetadata);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct StagingReceipt(String);

impl StagingReceipt {
    pub fn parse(value: impl Into<String>) -> Result<Self, SecretMigrationError> {
        let value = value.into();
        if !valid_prefixed_identifier(&value, "stage_", MAX_STAGING_RECEIPT_LEN) {
            return Err(SecretMigrationError::InvalidMetadata);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Opaque journal claim used to serialize one migration across processes.
///
/// The token is coordination metadata, not credential material, and is never
/// included in `Debug` output.
#[derive(Clone, PartialEq, Eq)]
pub struct MigrationLease {
    migration_id: SecretMigrationId,
    token: String,
    expires_at_ms: u64,
}

impl MigrationLease {
    pub fn new(
        migration_id: SecretMigrationId,
        token: impl Into<String>,
        expires_at_ms: u64,
    ) -> Result<Self, SecretMigrationError> {
        let token = token.into();
        if token.trim().is_empty()
            || token.len() > MAX_MIGRATION_LEASE_TOKEN_LEN
            || token.chars().any(char::is_control)
            || expires_at_ms == 0
        {
            return Err(SecretMigrationError::InvalidMetadata);
        }
        Ok(Self {
            migration_id,
            token,
            expires_at_ms,
        })
    }

    pub fn migration_id(&self) -> &SecretMigrationId {
        &self.migration_id
    }

    pub fn expires_at_ms(&self) -> u64 {
        self.expires_at_ms
    }
}

impl fmt::Debug for MigrationLease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MigrationLease")
            .field("migration_id", &self.migration_id)
            .field("token", &"[REDACTED]")
            .field("expires_at_ms", &self.expires_at_ms)
            .finish()
    }
}

/// A source category is metadata only; it never contains a path or secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacySourceKind {
    LegacyDatabase,
    ProviderConfig,
    PlaintextFile,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LegacySourceStatus {
    #[default]
    Available,
    Missing,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacySecretDescriptor {
    source_kind: LegacySourceKind,
    source_id: LegacySourceId,
    account: CredentialAccount,
    reference: CredentialRef,
}

impl LegacySecretDescriptor {
    pub fn new(
        source_kind: LegacySourceKind,
        source_id: LegacySourceId,
        account: CredentialAccount,
        reference: CredentialRef,
    ) -> Result<Self, SecretMigrationError> {
        if !valid_credential_reference(&reference) {
            return Err(SecretMigrationError::InvalidMetadata);
        }
        Ok(Self {
            source_kind,
            source_id,
            account,
            reference,
        })
    }

    pub fn source_kind(&self) -> LegacySourceKind {
        self.source_kind
    }

    pub fn source_id(&self) -> &LegacySourceId {
        &self.source_id
    }

    pub fn account(&self) -> &CredentialAccount {
        &self.account
    }

    pub fn reference(&self) -> &CredentialRef {
        &self.reference
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationDestination {
    account: CredentialAccount,
    reference: CredentialRef,
}

impl MigrationDestination {
    pub fn new(
        account: CredentialAccount,
        reference: CredentialRef,
    ) -> Result<Self, SecretMigrationError> {
        if !valid_credential_reference(&reference) {
            return Err(SecretMigrationError::InvalidMetadata);
        }
        Ok(Self { account, reference })
    }

    pub fn account(&self) -> &CredentialAccount {
        &self.account
    }

    pub fn reference(&self) -> &CredentialRef {
        &self.reference
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretMigrationAuthorization {
    actor_id: String,
    policy_revision: String,
    expires_at_ms: u64,
}

impl SecretMigrationAuthorization {
    pub fn new(
        actor_id: impl Into<String>,
        policy_revision: impl Into<String>,
        expires_at_ms: u64,
    ) -> Result<Self, SecretMigrationError> {
        let actor_id = actor_id.into();
        let policy_revision = policy_revision.into();
        if actor_id.trim().is_empty()
            || actor_id.len() > MAX_ACTOR_ID_LEN
            || actor_id.chars().any(char::is_control)
            || policy_revision.trim().is_empty()
            || policy_revision.len() > MAX_POLICY_REVISION_LEN
            || policy_revision.chars().any(char::is_control)
            || expires_at_ms == 0
        {
            return Err(SecretMigrationError::InvalidMetadata);
        }
        Ok(Self {
            actor_id,
            policy_revision,
            expires_at_ms,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretMigrationPolicy {
    authorization: SecretMigrationAuthorization,
    revoke_legacy: bool,
}

impl SecretMigrationPolicy {
    pub fn new(
        authorization: SecretMigrationAuthorization,
        revoke_legacy: bool,
    ) -> Result<Self, SecretMigrationError> {
        Ok(Self {
            authorization,
            revoke_legacy,
        })
    }

    pub fn policy_revision(&self) -> &str {
        &self.authorization.policy_revision
    }
}

#[derive(Debug, Clone)]
pub struct SecretMigrationRequest {
    pub migration_id: SecretMigrationId,
    pub context: CredentialAccessContext,
    pub source: LegacySecretDescriptor,
    pub destination: MigrationDestination,
    pub policy: SecretMigrationPolicy,
    retry_quarantined: bool,
}

impl SecretMigrationRequest {
    pub fn new(
        migration_id: SecretMigrationId,
        context: CredentialAccessContext,
        source: LegacySecretDescriptor,
        destination: MigrationDestination,
        policy: SecretMigrationPolicy,
    ) -> Result<Self, SecretMigrationError> {
        Ok(Self {
            migration_id,
            context,
            source,
            destination,
            policy,
            retry_quarantined: false,
        })
    }

    pub fn retry_quarantined(mut self) -> Self {
        self.retry_quarantined = true;
        self
    }
}

/// Authenticated ciphertext supplied by a trusted codec. Its bytes are never
/// printed by `Debug`; adapters are responsible for authenticated encryption.
#[derive(Clone, PartialEq, Eq)]
pub struct EncryptedSecretEnvelope {
    ciphertext: Vec<u8>,
}

impl EncryptedSecretEnvelope {
    pub fn from_sealed_bytes(bytes: Vec<u8>) -> Result<Self, SecretMigrationError> {
        if bytes.is_empty() || bytes.len() > MAX_STAGED_CIPHERTEXT_BYTES {
            return Err(SecretMigrationError::InvalidMetadata);
        }
        Ok(Self { ciphertext: bytes })
    }

    pub fn ciphertext(&self) -> &[u8] {
        &self.ciphertext
    }
}

impl fmt::Debug for EncryptedSecretEnvelope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EncryptedSecretEnvelope")
            .field("ciphertext", &"[REDACTED]")
            .field("length", &self.ciphertext.len())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationState {
    Started,
    Staged,
    DestinationWritten,
    Verified,
    Applied,
    Quarantined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationFailureClass {
    SourceInspect,
    SourceRead,
    SourceRevoke,
    CodecSeal,
    CodecOpen,
    StageWrite,
    StageRead,
    DestinationWrite,
    DestinationVerify,
    LedgerWrite,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationRecord {
    id: SecretMigrationId,
    source: LegacySecretDescriptor,
    destination: MigrationDestination,
    policy_revision: String,
    state: MigrationState,
    staging_receipt: Option<StagingReceipt>,
    failure: Option<MigrationFailureClass>,
    cleanup_pending: bool,
}

impl MigrationRecord {
    fn new(request: &SecretMigrationRequest) -> Self {
        Self {
            id: request.migration_id.clone(),
            source: request.source.clone(),
            destination: request.destination.clone(),
            policy_revision: request.policy.policy_revision().to_owned(),
            state: MigrationState::Started,
            staging_receipt: None,
            failure: None,
            cleanup_pending: false,
        }
    }

    pub fn id(&self) -> &SecretMigrationId {
        &self.id
    }

    pub fn state(&self) -> MigrationState {
        self.state
    }

    pub fn staging_receipt(&self) -> Option<&StagingReceipt> {
        self.staging_receipt.as_ref()
    }

    pub fn failure(&self) -> Option<MigrationFailureClass> {
        self.failure
    }

    pub fn cleanup_pending(&self) -> bool {
        self.cleanup_pending
    }

    fn matches_request(&self, request: &SecretMigrationRequest) -> bool {
        self.id == request.migration_id
            && self.source == request.source
            && self.destination == request.destination
            && self.policy_revision == request.policy.policy_revision()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationClaim {
    Acquired {
        record: MigrationRecord,
        lease: MigrationLease,
    },
    AlreadyApplied {
        record: MigrationRecord,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationDisposition {
    Applied,
    AlreadyApplied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretMigrationResult {
    pub disposition: MigrationDisposition,
    pub record: MigrationRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SecretMigrationError {
    #[error("secret migration metadata is invalid")]
    InvalidMetadata,
    #[error("secret migration scope is unauthorized")]
    Unauthorized,
    #[error("secret migration authorization expired")]
    AuthorizationExpired,
    #[error("secret migration requires explicit legacy-revocation consent")]
    ConsentRequired,
    #[error("secret migration was cancelled")]
    Cancelled,
    #[error("secret migration operation conflicts with an existing record")]
    Conflict,
    #[error("secret migration legacy source is unavailable")]
    SourceUnavailable,
    #[error("secret migration legacy source is missing or revoked")]
    SourceMissing,
    #[error("secret migration is quarantined")]
    Quarantined { failure: MigrationFailureClass },
    #[error("secret migration step failed: {0:?}")]
    Step(MigrationFailureClass),
    #[error("secret migration journal is unavailable")]
    LedgerUnavailable,
    #[error("secret destination rejected the operation: {0}")]
    Destination(#[source] SecretStoreError),
}

/// Monotonic time supplied by the composition root; the core has no clock.
pub trait MigrationClock: Send + Sync {
    fn now_ms(&self) -> u64;
}

/// Legacy source inspection and effect port. `inspect` must not read material.
/// `revoke` is atomic from the coordinator's perspective: an error must leave
/// the source available and retryable, while success must mean the source is
/// revoked. Concrete adapters must enforce and test that contract.
pub trait LegacySecretSource: Send + Sync {
    fn inspect(
        &self,
        descriptor: &LegacySecretDescriptor,
    ) -> Result<LegacySourceStatus, SecretMigrationError>;
    fn read(
        &self,
        descriptor: &LegacySecretDescriptor,
    ) -> Result<SecretMaterial, SecretMigrationError>;
    fn revoke(&self, descriptor: &LegacySecretDescriptor) -> Result<(), SecretMigrationError>;
}

/// Trusted authenticated-encryption port. Implementations must not persist
/// the plaintext passed to `seal`; this crate only sees the opaque envelope.
pub trait SecretEnvelopeCodec: Send + Sync {
    fn seal(
        &self,
        migration_id: &SecretMigrationId,
        material: SecretMaterial,
    ) -> Result<EncryptedSecretEnvelope, SecretMigrationError>;
    fn open(
        &self,
        envelope: &EncryptedSecretEnvelope,
    ) -> Result<SecretMaterial, SecretMigrationError>;
}

/// Encrypted staging port. Receipts contain metadata only and are safe for a
/// durable journal; the staged bytes remain behind this port.
pub trait EncryptedSecretStaging: Send + Sync {
    fn put(
        &self,
        migration_id: &SecretMigrationId,
        envelope: EncryptedSecretEnvelope,
    ) -> Result<StagingReceipt, SecretMigrationError>;
    fn get(
        &self,
        receipt: &StagingReceipt,
    ) -> Result<EncryptedSecretEnvelope, SecretMigrationError>;
    fn remove(&self, receipt: &StagingReceipt) -> Result<(), SecretMigrationError>;
}

/// Destination broker port. Verification receives expected material only so a
/// broker can compare it locally; it returns only a boolean and never returns
/// destination material to the coordinator.
pub trait SecretMigrationDestination: Send + Sync {
    fn put(
        &self,
        context: &CredentialAccessContext,
        account: &CredentialAccount,
        reference: &CredentialRef,
        material: SecretMaterial,
    ) -> Result<(), SecretMigrationError>;
    fn verify(
        &self,
        context: &CredentialAccessContext,
        account: &CredentialAccount,
        reference: &CredentialRef,
        expected: &SecretMaterial,
    ) -> Result<bool, SecretMigrationError>;
}

/// Durable metadata journal. Implementations must atomically make `start`
/// idempotent and must never persist material or envelope bytes. `claim` must
/// atomically acquire an expiring exclusive lease for a non-terminal record;
/// an active lease returns `Conflict`, while an expired lease permits restart.
/// `save` is a compare-and-set guarded by the lease and `now_ms`; `release`
/// clears only the caller's lease. A process interrupted after claiming is
/// therefore recoverable after the bounded lease expires.
pub trait MigrationLedger: Send + Sync {
    fn load(
        &self,
        migration_id: &SecretMigrationId,
    ) -> Result<Option<MigrationRecord>, SecretMigrationError>;
    fn start(&self, record: MigrationRecord) -> Result<MigrationRecord, SecretMigrationError>;
    fn claim(
        &self,
        migration_id: &SecretMigrationId,
        now_ms: u64,
    ) -> Result<MigrationClaim, SecretMigrationError>;
    fn save(
        &self,
        lease: &MigrationLease,
        now_ms: u64,
        record: MigrationRecord,
    ) -> Result<(), SecretMigrationError>;
    fn release(&self, lease: &MigrationLease) -> Result<(), SecretMigrationError>;
}

pub struct SecretMigrationCoordinator {
    clock: Arc<dyn MigrationClock>,
    source: Arc<dyn LegacySecretSource>,
    codec: Arc<dyn SecretEnvelopeCodec>,
    staging: Arc<dyn EncryptedSecretStaging>,
    destination: Arc<dyn SecretMigrationDestination>,
    ledger: Arc<dyn MigrationLedger>,
}

impl SecretMigrationCoordinator {
    pub fn new(
        clock: Arc<dyn MigrationClock>,
        source: Arc<dyn LegacySecretSource>,
        codec: Arc<dyn SecretEnvelopeCodec>,
        staging: Arc<dyn EncryptedSecretStaging>,
        destination: Arc<dyn SecretMigrationDestination>,
        ledger: Arc<dyn MigrationLedger>,
    ) -> Self {
        Self {
            clock,
            source,
            codec,
            staging,
            destination,
            ledger,
        }
    }

    pub fn preflight(&self, request: &SecretMigrationRequest) -> Result<(), SecretMigrationError> {
        if request.context.cancellation.is_cancelled() {
            return Err(SecretMigrationError::Cancelled);
        }
        if request.context.project_id != request.source.account.project_id
            || request.context.project_id != request.destination.account.project_id
        {
            return Err(SecretMigrationError::Unauthorized);
        }
        if request.source.account == request.destination.account
            && request.source.reference == request.destination.reference
        {
            return Err(SecretMigrationError::InvalidMetadata);
        }
        if request.context.actor_id != request.policy.authorization.actor_id {
            return Err(SecretMigrationError::Unauthorized);
        }
        if self.clock.now_ms() >= request.policy.authorization.expires_at_ms {
            return Err(SecretMigrationError::AuthorizationExpired);
        }
        if !request.policy.revoke_legacy {
            return Err(SecretMigrationError::ConsentRequired);
        }
        Ok(())
    }

    pub fn migrate(
        &self,
        request: SecretMigrationRequest,
    ) -> Result<SecretMigrationResult, SecretMigrationError> {
        self.preflight(&request)?;
        match self.ledger.load(&request.migration_id)? {
            Some(record) => {
                if !record.matches_request(&request) {
                    return Err(SecretMigrationError::Conflict);
                }
                if record.state == MigrationState::Applied {
                    return Ok(SecretMigrationResult {
                        disposition: MigrationDisposition::AlreadyApplied,
                        record,
                    });
                }
                if record.state == MigrationState::Quarantined && !request.retry_quarantined {
                    return Err(SecretMigrationError::Quarantined {
                        failure: record.failure.unwrap_or(MigrationFailureClass::LedgerWrite),
                    });
                }
            }
            None => {
                let started = MigrationRecord::new(&request);
                let record = self
                    .ledger
                    .start(started)
                    .map_err(|_| SecretMigrationError::LedgerUnavailable)?;
                if !record.matches_request(&request) {
                    return Err(SecretMigrationError::Conflict);
                }
            }
        };

        let (mut record, lease) = match self
            .ledger
            .claim(&request.migration_id, self.clock.now_ms())?
        {
            MigrationClaim::AlreadyApplied { record } => {
                if !record.matches_request(&request) {
                    return Err(SecretMigrationError::Conflict);
                }
                return Ok(SecretMigrationResult {
                    disposition: MigrationDisposition::AlreadyApplied,
                    record,
                });
            }
            MigrationClaim::Acquired { record, lease } => (record, lease),
        };
        if !record.matches_request(&request) {
            self.release(&lease)?;
            return Err(SecretMigrationError::Conflict);
        }
        if record.state == MigrationState::Quarantined {
            if !request.retry_quarantined {
                let failure = record.failure.unwrap_or(MigrationFailureClass::LedgerWrite);
                self.release(&lease)?;
                return Err(SecretMigrationError::Quarantined { failure });
            }
            record.state = if record.staging_receipt.is_some() {
                MigrationState::Staged
            } else {
                MigrationState::Started
            };
            record.failure = None;
            self.save(&lease, &record)?;
        }

        if record.staging_receipt.is_none() {
            match self.source.inspect(&record.source) {
                Err(_) => {
                    return self.quarantine(&lease, record, MigrationFailureClass::SourceInspect)
                }
                Ok(LegacySourceStatus::Available) => {}
                Ok(LegacySourceStatus::Missing | LegacySourceStatus::Revoked) => {
                    return self.quarantine(&lease, record, MigrationFailureClass::SourceInspect);
                }
            }
            let material = match self.source.read(&record.source) {
                Ok(material) => material,
                Err(_) => {
                    return self.quarantine(&lease, record, MigrationFailureClass::SourceRead)
                }
            };
            let envelope = match self.codec.seal(&record.id, material) {
                Ok(envelope) => envelope,
                Err(_) => return self.quarantine(&lease, record, MigrationFailureClass::CodecSeal),
            };
            let receipt = match self.staging.put(&record.id, envelope) {
                Ok(receipt) => receipt,
                Err(_) => {
                    return self.quarantine(&lease, record, MigrationFailureClass::StageWrite)
                }
            };
            record.staging_receipt = Some(receipt);
            record.state = MigrationState::Staged;
            self.save(&lease, &record)?;
        }

        let receipt = record
            .staging_receipt
            .clone()
            .ok_or(SecretMigrationError::Step(MigrationFailureClass::StageRead))?;
        let envelope = match self.staging.get(&receipt) {
            Ok(envelope) => envelope,
            Err(_) => return self.quarantine(&lease, record, MigrationFailureClass::StageRead),
        };
        let material = match self.codec.open(&envelope) {
            Ok(material) => material,
            Err(_) => return self.quarantine(&lease, record, MigrationFailureClass::CodecOpen),
        };
        if self
            .destination
            .put(
                &request.context,
                &record.destination.account,
                &record.destination.reference,
                material.clone(),
            )
            .is_err()
        {
            return self.quarantine(&lease, record, MigrationFailureClass::DestinationWrite);
        }
        record.state = MigrationState::DestinationWritten;
        self.save(&lease, &record)?;

        let verified = match self.destination.verify(
            &request.context,
            &record.destination.account,
            &record.destination.reference,
            &material,
        ) {
            Ok(verified) => verified,
            Err(_) => {
                return self.quarantine(&lease, record, MigrationFailureClass::DestinationVerify)
            }
        };
        if !verified {
            return self.quarantine(&lease, record, MigrationFailureClass::DestinationVerify);
        }
        record.state = MigrationState::Verified;
        self.save(&lease, &record)?;

        if self.source.revoke(&record.source).is_err() {
            return self.quarantine(&lease, record, MigrationFailureClass::SourceRevoke);
        }
        record.state = MigrationState::Applied;
        record.failure = None;
        self.save(&lease, &record)?;

        if self.staging.remove(&receipt).is_err() {
            record.cleanup_pending = true;
            self.save(&lease, &record)?;
        }
        self.release(&lease)?;
        Ok(SecretMigrationResult {
            disposition: MigrationDisposition::Applied,
            record,
        })
    }

    fn save(
        &self,
        lease: &MigrationLease,
        record: &MigrationRecord,
    ) -> Result<(), SecretMigrationError> {
        self.ledger
            .save(lease, self.clock.now_ms(), record.clone())
            .map_err(|_| SecretMigrationError::LedgerUnavailable)
    }

    fn release(&self, lease: &MigrationLease) -> Result<(), SecretMigrationError> {
        self.ledger
            .release(lease)
            .map_err(|_| SecretMigrationError::LedgerUnavailable)
    }

    fn quarantine(
        &self,
        lease: &MigrationLease,
        mut record: MigrationRecord,
        failure: MigrationFailureClass,
    ) -> Result<SecretMigrationResult, SecretMigrationError> {
        record.state = MigrationState::Quarantined;
        record.failure = Some(failure);
        self.save(lease, &record)?;
        self.release(lease)?;
        Err(SecretMigrationError::Quarantined { failure })
    }
}

impl<B: SecureSecretBackend> SecretMigrationDestination for SecureSecretStore<B> {
    fn put(
        &self,
        context: &CredentialAccessContext,
        account: &CredentialAccount,
        reference: &CredentialRef,
        material: SecretMaterial,
    ) -> Result<(), SecretMigrationError> {
        self.put(
            context.clone(),
            account.clone(),
            reference.clone(),
            material,
        )
        .map_err(SecretMigrationError::Destination)
    }

    fn verify(
        &self,
        context: &CredentialAccessContext,
        account: &CredentialAccount,
        reference: &CredentialRef,
        expected: &SecretMaterial,
    ) -> Result<bool, SecretMigrationError> {
        let stored = self
            .get(context.clone(), account.clone(), reference.clone())
            .map_err(SecretMigrationError::Destination)?;
        let length_matches = stored.as_bytes().len() == expected.as_bytes().len();
        let bytes_match = stored.as_bytes().ct_eq(expected.as_bytes()).unwrap_u8() == 1;
        Ok(length_matches && bytes_match)
    }
}

fn valid_prefixed_identifier(value: &str, prefix: &str, max_len: usize) -> bool {
    value.starts_with(prefix)
        && value.len() > prefix.len()
        && value.len() <= max_len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn valid_credential_reference(reference: &CredentialRef) -> bool {
    let value = reference.as_str();
    let normalized = value.to_ascii_lowercase();
    value.starts_with("cred_")
        && value.len() > 5
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        && !["api_key", "secret", "token", "password", "bearer"]
            .iter()
            .any(|marker| normalized.contains(marker))
}
