use provider_core::credentials::{CredentialAccessContext, CredentialAccount, ProjectScopeId};
use provider_core::{CancellationToken, CredentialRef, ProviderId};
use secrets_core::{
    BackendKind, BackendStatus, SecretMaterial, SecretStoreError, SecureSecretBackend,
    SecureSecretStore,
};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct MockKeychain {
    state: Arc<Mutex<MockState>>,
}

#[derive(Default)]
struct MockState {
    available: bool,
    records: BTreeMap<String, (CredentialAccount, Vec<u8>)>,
}

impl MockKeychain {
    fn available() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState {
                available: true,
                records: BTreeMap::new(),
            })),
        }
    }

    fn unavailable() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState::default())),
        }
    }
}

impl SecureSecretBackend for MockKeychain {
    fn kind(&self) -> BackendKind {
        BackendKind::OsKeychain
    }

    fn status(&self) -> BackendStatus {
        if self.state.lock().unwrap().available {
            BackendStatus::Available
        } else {
            BackendStatus::Unavailable
        }
    }

    fn put(
        &self,
        reference: &CredentialRef,
        account: &CredentialAccount,
        material: SecretMaterial,
    ) -> Result<(), SecretStoreError> {
        let mut state = self.state.lock().unwrap();
        if !state.available {
            return Err(SecretStoreError::Unavailable);
        }
        state.records.insert(
            reference.as_str().to_string(),
            (account.clone(), material.into_bytes()),
        );
        Ok(())
    }

    fn get(
        &self,
        reference: &CredentialRef,
        account: &CredentialAccount,
    ) -> Result<SecretMaterial, SecretStoreError> {
        let state = self.state.lock().unwrap();
        let (stored_account, bytes) = state
            .records
            .get(reference.as_str())
            .ok_or(SecretStoreError::Missing)?;
        if stored_account != account {
            return Err(SecretStoreError::Unauthorized);
        }
        Ok(SecretMaterial::new(bytes.clone()).unwrap())
    }

    fn delete(
        &self,
        reference: &CredentialRef,
        account: &CredentialAccount,
    ) -> Result<(), SecretStoreError> {
        let mut state = self.state.lock().unwrap();
        let (stored_account, _) = state
            .records
            .get(reference.as_str())
            .ok_or(SecretStoreError::Missing)?;
        if stored_account != account {
            return Err(SecretStoreError::Unauthorized);
        }
        state.records.remove(reference.as_str());
        Ok(())
    }

    fn rotate(
        &self,
        reference: &CredentialRef,
        account: &CredentialAccount,
        material: SecretMaterial,
    ) -> Result<(), SecretStoreError> {
        self.put(reference, account, material)
    }
}

fn account(project: &str) -> CredentialAccount {
    CredentialAccount::new(
        ProjectScopeId::parse(project).unwrap(),
        ProviderId::parse("openai").unwrap(),
        provider_core::credentials::AccountId::parse("account_1").unwrap(),
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

#[test]
fn secret_roundtrip_uses_mock_keychain_and_opaque_ref() {
    let store = SecureSecretStore::new(MockKeychain::available());
    let account = account("project_1");
    let reference = CredentialRef::parse("cred_openai_1").unwrap();
    store
        .put(
            context("project_1"),
            account.clone(),
            reference.clone(),
            SecretMaterial::new(b"[TEST-SECRET]".to_vec()).unwrap(),
        )
        .unwrap();
    let material = store.get(context("project_1"), account, reference).unwrap();
    assert_eq!(material.as_bytes(), b"[TEST-SECRET]");
}

#[test]
fn wrong_project_scope_fails_without_reading_secret() {
    let store = SecureSecretStore::new(MockKeychain::available());
    let account = account("project_1");
    let reference = CredentialRef::parse("cred_openai_1").unwrap();
    store
        .put(
            context("project_1"),
            account.clone(),
            reference.clone(),
            SecretMaterial::new(b"[TEST-SECRET]".to_vec()).unwrap(),
        )
        .unwrap();
    let err = store
        .get(context("project_2"), account, reference)
        .unwrap_err();
    assert!(matches!(err, SecretStoreError::Unauthorized));
    assert!(!err.to_string().contains("[TEST-SECRET]"));
}

#[test]
fn delete_and_revoke_are_idempotent_at_service_boundary() {
    let store = SecureSecretStore::new(MockKeychain::available());
    let account = account("project_1");
    let reference = CredentialRef::parse("cred_openai_1").unwrap();
    store
        .put(
            context("project_1"),
            account.clone(),
            reference.clone(),
            SecretMaterial::new(b"[TEST-SECRET]".to_vec()).unwrap(),
        )
        .unwrap();
    store
        .delete(context("project_1"), account.clone(), reference.clone())
        .unwrap();
    assert!(matches!(
        store.delete(context("project_1"), account, reference),
        Err(SecretStoreError::Missing)
    ));
}

#[test]
fn rotate_replaces_secret_without_changing_opaque_ref() {
    let store = SecureSecretStore::new(MockKeychain::available());
    let account = account("project_1");
    let reference = CredentialRef::parse("cred_openai_1").unwrap();
    store
        .put(
            context("project_1"),
            account.clone(),
            reference.clone(),
            SecretMaterial::new(b"[OLD-SECRET]".to_vec()).unwrap(),
        )
        .unwrap();
    store
        .rotate(
            context("project_1"),
            account.clone(),
            reference.clone(),
            SecretMaterial::new(b"[NEW-SECRET]".to_vec()).unwrap(),
        )
        .unwrap();
    assert_eq!(
        store
            .get(context("project_1"), account, reference)
            .unwrap()
            .as_bytes(),
        b"[NEW-SECRET]"
    );
}

#[test]
fn wrong_account_binding_fails_closed() {
    let store = SecureSecretStore::new(MockKeychain::available());
    let stored_account = account("project_1");
    let other_account = CredentialAccount::new(
        ProjectScopeId::parse("project_1").unwrap(),
        ProviderId::parse("openai").unwrap(),
        provider_core::credentials::AccountId::parse("account_2").unwrap(),
    )
    .unwrap();
    let reference = CredentialRef::parse("cred_openai_1").unwrap();
    store
        .put(
            context("project_1"),
            stored_account,
            reference.clone(),
            SecretMaterial::new(b"[TEST-SECRET]".to_vec()).unwrap(),
        )
        .unwrap();
    let err = store
        .get(context("project_1"), other_account, reference)
        .unwrap_err();
    assert!(matches!(err, SecretStoreError::Unauthorized));
}

#[test]
fn unavailable_backend_fails_closed_without_plaintext_fallback() {
    let store = SecureSecretStore::new(MockKeychain::unavailable());
    assert_eq!(store.backend_status(), BackendStatus::Unavailable);
    let err = store
        .put(
            context("project_1"),
            account("project_1"),
            CredentialRef::parse("cred_openai_1").unwrap(),
            SecretMaterial::new(b"[TEST-SECRET]".to_vec()).unwrap(),
        )
        .unwrap_err();
    assert!(matches!(err, SecretStoreError::Unavailable));
}

#[test]
fn malformed_and_oversized_material_fail_closed() {
    assert!(SecretMaterial::new(Vec::new()).is_err());
    assert!(SecretMaterial::new(vec![0_u8; secrets_core::MAX_SECRET_BYTES + 1]).is_err());
    assert!(CredentialRef::parse("api_key=plaintext").is_err());
}

#[test]
fn cancelled_operations_fail_before_backend_call() {
    let store = SecureSecretStore::new(MockKeychain::available());
    let token = CancellationToken::new();
    token.cancel();
    let cancelled = CredentialAccessContext::new(
        ProjectScopeId::parse("project_1").unwrap(),
        "agent_1".into(),
        token,
    )
    .unwrap();
    let err = store
        .put(
            cancelled,
            account("project_1"),
            CredentialRef::parse("cred_openai_1").unwrap(),
            SecretMaterial::new(b"[TEST-SECRET]".to_vec()).unwrap(),
        )
        .unwrap_err();
    assert!(matches!(err, SecretStoreError::Cancelled));
}
