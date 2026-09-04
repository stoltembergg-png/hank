use provider_core::credentials::{CredentialAccessContext, CredentialAccount, ProjectScopeId};
use provider_core::{CancellationToken, CredentialRef, ProviderId};
use secrets_core::migration::{
    EncryptedSecretEnvelope, EncryptedSecretStaging, LegacySecretDescriptor, LegacySecretSource,
    LegacySourceId, LegacySourceKind, LegacySourceStatus, MigrationClock, MigrationDisposition,
    MigrationFailureClass, MigrationLedger, MigrationRecord, MigrationState, SecretEnvelopeCodec,
    SecretMigrationAuthorization, SecretMigrationCoordinator, SecretMigrationDestination,
    SecretMigrationError, SecretMigrationId, SecretMigrationPolicy, SecretMigrationRequest,
    StagingReceipt,
};
use secrets_core::SecretMaterial;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const TEST_SECRET: &[u8] = b"[TEST-SECRET]";

fn account(project: &str, provider: &str, id: &str) -> CredentialAccount {
    CredentialAccount::new(
        ProjectScopeId::parse(project).unwrap(),
        ProviderId::parse(provider).unwrap(),
        provider_core::credentials::AccountId::parse(id).unwrap(),
    )
    .unwrap()
}

fn context(project: &str) -> CredentialAccessContext {
    CredentialAccessContext::new(
        ProjectScopeId::parse(project).unwrap(),
        "agent_1".into(),
        CancellationToken::new(),
    )
    .unwrap()
}

fn request(destination_project: &str) -> SecretMigrationRequest {
    let source = LegacySecretDescriptor::new(
        LegacySourceKind::LegacyDatabase,
        LegacySourceId::parse("legacy_source_1").unwrap(),
        account("project_1", "openai", "account_1"),
        CredentialRef::parse("cred_legacy_1").unwrap(),
    )
    .unwrap();
    let destination = secrets_core::migration::MigrationDestination::new(
        account(destination_project, "openai", "account_2"),
        CredentialRef::parse("cred_destination_1").unwrap(),
    )
    .unwrap();
    let authorization = SecretMigrationAuthorization::new("agent_1", "policy_1", 2_000).unwrap();
    SecretMigrationRequest::new(
        SecretMigrationId::parse("migration_1").unwrap(),
        context("project_1"),
        source,
        destination,
        SecretMigrationPolicy::new(authorization, true).unwrap(),
    )
    .unwrap()
}

#[derive(Default)]
struct FixedClock {
    now_ms: AtomicU64,
}

impl FixedClock {
    fn new(now_ms: u64) -> Arc<Self> {
        Arc::new(Self {
            now_ms: AtomicU64::new(now_ms),
        })
    }
}

impl MigrationClock for FixedClock {
    fn now_ms(&self) -> u64 {
        self.now_ms.load(Ordering::SeqCst)
    }
}

#[derive(Default)]
struct LegacyState {
    status: LegacySourceStatus,
    material: Vec<u8>,
    inspect_calls: usize,
    read_calls: usize,
    revoke_calls: usize,
    fail_revoke: bool,
}

struct MockLegacySource {
    state: Arc<Mutex<LegacyState>>,
}

impl MockLegacySource {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Arc::new(Mutex::new(LegacyState {
                status: LegacySourceStatus::Available,
                material: TEST_SECRET.to_vec(),
                ..LegacyState::default()
            })),
        })
    }
}

impl LegacySecretSource for MockLegacySource {
    fn inspect(
        &self,
        _descriptor: &LegacySecretDescriptor,
    ) -> Result<LegacySourceStatus, SecretMigrationError> {
        let mut state = self.state.lock().unwrap();
        state.inspect_calls += 1;
        Ok(state.status)
    }

    fn read(
        &self,
        _descriptor: &LegacySecretDescriptor,
    ) -> Result<SecretMaterial, SecretMigrationError> {
        let mut state = self.state.lock().unwrap();
        state.read_calls += 1;
        SecretMaterial::new(state.material.clone())
            .map_err(|_| SecretMigrationError::Step(MigrationFailureClass::SourceRead))
    }

    fn revoke(&self, _descriptor: &LegacySecretDescriptor) -> Result<(), SecretMigrationError> {
        let mut state = self.state.lock().unwrap();
        state.revoke_calls += 1;
        if state.fail_revoke {
            return Err(SecretMigrationError::Step(
                MigrationFailureClass::SourceRevoke,
            ));
        }
        state.status = LegacySourceStatus::Revoked;
        Ok(())
    }
}

#[derive(Default)]
struct CodecState {
    next_id: u64,
    materials: BTreeMap<Vec<u8>, Vec<u8>>,
    envelopes: Vec<EncryptedSecretEnvelope>,
}

struct MockCodec {
    state: Arc<Mutex<CodecState>>,
}

impl MockCodec {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Arc::new(Mutex::new(CodecState::default())),
        })
    }
}

impl SecretEnvelopeCodec for MockCodec {
    fn seal(
        &self,
        _migration_id: &SecretMigrationId,
        material: SecretMaterial,
    ) -> Result<EncryptedSecretEnvelope, SecretMigrationError> {
        let mut state = self.state.lock().unwrap();
        state.next_id += 1;
        let ciphertext = vec![0xA5, (state.next_id & 0xFF) as u8];
        state
            .materials
            .insert(ciphertext.clone(), material.into_bytes());
        let envelope = EncryptedSecretEnvelope::from_sealed_bytes(ciphertext)
            .map_err(|_| SecretMigrationError::Step(MigrationFailureClass::CodecSeal))?;
        state.envelopes.push(envelope.clone());
        Ok(envelope)
    }

    fn open(
        &self,
        envelope: &EncryptedSecretEnvelope,
    ) -> Result<SecretMaterial, SecretMigrationError> {
        let state = self.state.lock().unwrap();
        let material = state
            .materials
            .get(envelope.ciphertext())
            .cloned()
            .ok_or(SecretMigrationError::Step(MigrationFailureClass::CodecOpen))?;
        SecretMaterial::new(material)
            .map_err(|_| SecretMigrationError::Step(MigrationFailureClass::CodecOpen))
    }
}

#[derive(Default)]
struct StagingState {
    next_id: u64,
    entries: BTreeMap<String, EncryptedSecretEnvelope>,
    seen: Vec<EncryptedSecretEnvelope>,
    fail_put: bool,
}

struct MockStaging {
    state: Arc<Mutex<StagingState>>,
}

impl MockStaging {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Arc::new(Mutex::new(StagingState::default())),
        })
    }
}

impl EncryptedSecretStaging for MockStaging {
    fn put(
        &self,
        _migration_id: &SecretMigrationId,
        envelope: EncryptedSecretEnvelope,
    ) -> Result<StagingReceipt, SecretMigrationError> {
        let mut state = self.state.lock().unwrap();
        if state.fail_put {
            return Err(SecretMigrationError::Step(
                MigrationFailureClass::StageWrite,
            ));
        }
        state.next_id += 1;
        let receipt = StagingReceipt::parse(format!("stage_{}", state.next_id)).unwrap();
        state.seen.push(envelope.clone());
        state.entries.insert(receipt.as_str().to_owned(), envelope);
        Ok(receipt)
    }

    fn get(
        &self,
        receipt: &StagingReceipt,
    ) -> Result<EncryptedSecretEnvelope, SecretMigrationError> {
        self.state
            .lock()
            .unwrap()
            .entries
            .get(receipt.as_str())
            .cloned()
            .ok_or(SecretMigrationError::Step(MigrationFailureClass::StageRead))
    }

    fn remove(&self, receipt: &StagingReceipt) -> Result<(), SecretMigrationError> {
        self.state.lock().unwrap().entries.remove(receipt.as_str());
        Ok(())
    }
}

#[derive(Default)]
struct DestinationState {
    records: BTreeMap<String, Vec<u8>>,
    write_calls: usize,
    verify_calls: usize,
    fail_write: bool,
    verify_result: bool,
}

struct MockDestination {
    state: Arc<Mutex<DestinationState>>,
}

impl MockDestination {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Arc::new(Mutex::new(DestinationState {
                verify_result: true,
                ..DestinationState::default()
            })),
        })
    }
}

impl SecretMigrationDestination for MockDestination {
    fn put(
        &self,
        _context: &CredentialAccessContext,
        _account: &CredentialAccount,
        reference: &CredentialRef,
        material: SecretMaterial,
    ) -> Result<(), SecretMigrationError> {
        let mut state = self.state.lock().unwrap();
        if state.fail_write {
            return Err(SecretMigrationError::Step(
                MigrationFailureClass::DestinationWrite,
            ));
        }
        state.write_calls += 1;
        state
            .records
            .insert(reference.as_str().to_owned(), material.into_bytes());
        Ok(())
    }

    fn verify(
        &self,
        _context: &CredentialAccessContext,
        _account: &CredentialAccount,
        _reference: &CredentialRef,
        _expected: &SecretMaterial,
    ) -> Result<bool, SecretMigrationError> {
        let mut state = self.state.lock().unwrap();
        state.verify_calls += 1;
        Ok(state.verify_result)
    }
}

#[derive(Default)]
struct MockLedger {
    records: Mutex<BTreeMap<SecretMigrationId, MigrationRecord>>,
}

impl MigrationLedger for MockLedger {
    fn load(
        &self,
        migration_id: &SecretMigrationId,
    ) -> Result<Option<MigrationRecord>, SecretMigrationError> {
        Ok(self.records.lock().unwrap().get(migration_id).cloned())
    }

    fn start(&self, record: MigrationRecord) -> Result<MigrationRecord, SecretMigrationError> {
        let mut records = self.records.lock().unwrap();
        Ok(records.entry(record.id().clone()).or_insert(record).clone())
    }

    fn save(&self, record: MigrationRecord) -> Result<(), SecretMigrationError> {
        self.records
            .lock()
            .unwrap()
            .insert(record.id().clone(), record);
        Ok(())
    }
}

type CoordinatorFixture = (
    SecretMigrationCoordinator,
    Arc<MockLegacySource>,
    Arc<MockCodec>,
    Arc<MockStaging>,
    Arc<MockDestination>,
    Arc<MockLedger>,
);

fn coordinator() -> CoordinatorFixture {
    let source = MockLegacySource::new();
    let codec = MockCodec::new();
    let staging = MockStaging::new();
    let destination = MockDestination::new();
    let ledger = Arc::new(MockLedger::default());
    let coordinator = SecretMigrationCoordinator::new(
        FixedClock::new(1_000),
        source.clone(),
        codec.clone(),
        staging.clone(),
        destination.clone(),
        ledger.clone(),
    );
    (coordinator, source, codec, staging, destination, ledger)
}

#[test]
// @spec:AC-1902
fn cross_project_destination_is_rejected_before_legacy_read() {
    let (coordinator, source, _codec, _staging, _destination, ledger) = coordinator();
    let error = coordinator.migrate(request("project_2")).unwrap_err();
    assert!(matches!(error, SecretMigrationError::Unauthorized));
    assert_eq!(source.state.lock().unwrap().inspect_calls, 0);
    assert_eq!(source.state.lock().unwrap().read_calls, 0);
    assert!(ledger.records.lock().unwrap().is_empty());
}

#[test]
// @spec:AC-1901
// @spec:AC-1903
// @spec:AC-1904
// @spec:AC-1907
fn successful_migration_stages_verifies_and_revokes_after_cutover() {
    let (coordinator, source, codec, staging, destination, _ledger) = coordinator();
    let result = coordinator.migrate(request("project_1")).unwrap();
    assert_eq!(result.disposition, MigrationDisposition::Applied);
    assert_eq!(result.record.state(), MigrationState::Applied);
    assert_eq!(source.state.lock().unwrap().inspect_calls, 1);
    assert_eq!(source.state.lock().unwrap().read_calls, 1);
    assert_eq!(source.state.lock().unwrap().revoke_calls, 1);
    assert_eq!(destination.state.lock().unwrap().write_calls, 1);
    assert_eq!(destination.state.lock().unwrap().verify_calls, 1);
    assert!(staging.state.lock().unwrap().entries.is_empty());
    assert_eq!(codec.state.lock().unwrap().envelopes.len(), 1);
    assert!(!codec.state.lock().unwrap().envelopes[0]
        .ciphertext()
        .windows(TEST_SECRET.len())
        .any(|window| window == TEST_SECRET));
    assert!(!format!("{result:?}").contains("TEST-SECRET"));
}

#[test]
// @spec:AC-1904
// @spec:AC-1905
// @spec:AC-1908
fn verification_failure_quarantines_and_preserves_legacy_source() {
    let (coordinator, source, _codec, staging, destination, ledger) = coordinator();
    destination.state.lock().unwrap().verify_result = false;
    let error = coordinator.migrate(request("project_1")).unwrap_err();
    assert!(matches!(
        error,
        SecretMigrationError::Quarantined {
            failure: MigrationFailureClass::DestinationVerify
        }
    ));
    let record = ledger
        .records
        .lock()
        .unwrap()
        .get(&SecretMigrationId::parse("migration_1").unwrap())
        .cloned()
        .unwrap();
    assert_eq!(record.state(), MigrationState::Quarantined);
    assert_eq!(source.state.lock().unwrap().revoke_calls, 0);
    assert_eq!(staging.state.lock().unwrap().entries.len(), 1);
    assert!(!format!("{error:?}").contains("TEST-SECRET"));
}

#[test]
// @spec:AC-1905
// @spec:AC-1907
fn revoke_failure_quarantines_and_keeps_legacy_source_recoverable() {
    let (coordinator, source, _codec, staging, _destination, ledger) = coordinator();
    source.state.lock().unwrap().fail_revoke = true;
    let error = coordinator.migrate(request("project_1")).unwrap_err();
    assert!(matches!(
        error,
        SecretMigrationError::Quarantined {
            failure: MigrationFailureClass::SourceRevoke
        }
    ));
    let record = ledger
        .records
        .lock()
        .unwrap()
        .get(&SecretMigrationId::parse("migration_1").unwrap())
        .cloned()
        .unwrap();
    assert_eq!(record.state(), MigrationState::Quarantined);
    assert_eq!(
        source.state.lock().unwrap().status,
        LegacySourceStatus::Available
    );
    assert_eq!(staging.state.lock().unwrap().entries.len(), 1);
}

#[test]
// @spec:AC-1905
// @spec:AC-1906
fn retry_resumes_from_encrypted_stage_without_rereading_legacy_source() {
    let (coordinator, source, _codec, staging, destination, _ledger) = coordinator();
    destination.state.lock().unwrap().fail_write = true;
    assert!(matches!(
        coordinator.migrate(request("project_1")),
        Err(SecretMigrationError::Quarantined { .. })
    ));
    assert_eq!(source.state.lock().unwrap().read_calls, 1);
    assert_eq!(staging.state.lock().unwrap().entries.len(), 1);

    destination.state.lock().unwrap().fail_write = false;
    let retry = request("project_1").retry_quarantined();
    let result = coordinator.migrate(retry).unwrap();
    assert_eq!(result.disposition, MigrationDisposition::Applied);
    assert_eq!(source.state.lock().unwrap().read_calls, 1);
    assert_eq!(source.state.lock().unwrap().revoke_calls, 1);
    assert!(staging.state.lock().unwrap().entries.is_empty());
}

#[test]
// @spec:AC-1906
fn duplicate_applied_migration_is_idempotent_without_second_cutover() {
    let (coordinator, source, _codec, _staging, destination, _ledger) = coordinator();
    let first = coordinator.migrate(request("project_1")).unwrap();
    let second = coordinator.migrate(request("project_1")).unwrap();
    assert_eq!(first.disposition, MigrationDisposition::Applied);
    assert_eq!(second.disposition, MigrationDisposition::AlreadyApplied);
    assert_eq!(source.state.lock().unwrap().read_calls, 1);
    assert_eq!(source.state.lock().unwrap().revoke_calls, 1);
    assert_eq!(destination.state.lock().unwrap().write_calls, 1);
}

#[test]
// @spec:AC-1902
fn expired_or_non_revoking_policy_is_rejected_before_source_inspection() {
    let (coordinator, source, _codec, _staging, _destination, _ledger) = coordinator();
    let mut expired = request("project_1");
    expired.policy = SecretMigrationPolicy::new(
        SecretMigrationAuthorization::new("agent_1", "policy_1", 999).unwrap(),
        true,
    )
    .unwrap();
    assert!(matches!(
        coordinator.migrate(expired),
        Err(SecretMigrationError::AuthorizationExpired)
    ));
    let mut no_revoke = request("project_1");
    no_revoke.policy = SecretMigrationPolicy::new(
        SecretMigrationAuthorization::new("agent_1", "policy_1", 2_000).unwrap(),
        false,
    )
    .unwrap();
    assert!(matches!(
        coordinator.migrate(no_revoke),
        Err(SecretMigrationError::ConsentRequired)
    ));
    assert_eq!(source.state.lock().unwrap().inspect_calls, 0);
}

#[test]
// @spec:AC-1903
// @spec:AC-1908
fn malformed_migration_metadata_and_envelope_fail_closed() {
    assert!(SecretMigrationId::parse("migration_").is_err());
    assert!(LegacySourceId::parse("source_1").is_err());
    assert!(EncryptedSecretEnvelope::from_sealed_bytes(Vec::new()).is_err());
    assert!(EncryptedSecretEnvelope::from_sealed_bytes(vec![
        0_u8;
        secrets_core::migration::MAX_STAGED_CIPHERTEXT_BYTES
            + 1
    ])
    .is_err());
}
