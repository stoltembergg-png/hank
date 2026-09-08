//! Durable project-scoped provider credential metadata with OS-secret storage.

use agent_runtime::SqliteStorage;
use provider_core::credentials::{
    AccountId, CredentialAccessContext, CredentialAccount, CredentialService,
    CredentialServiceError, CredentialServiceState, CredentialStatus, ProjectScopeId,
};
use provider_core::{CancellationToken, CredentialRef, ProviderId};
use secrets_core::{
    BackendKind, BackendStatus, SecretMaterial, SecretStoreError, SecureSecretBackend,
    SecureSecretStore,
};
use sqlx::{Pool, Row, Sqlite};
use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, RwLock};

const STATE_CONNECTED: &str = "connected";
const STATE_REVOKED: &str = "revoked";
const STATE_UNAVAILABLE: &str = "unavailable";
const STATE_ERROR: &str = "error";

#[derive(Debug, Clone)]
struct MetadataRecord {
    state: CredentialServiceState,
    credential_ref: Option<CredentialRef>,
}

/// Durable account metadata backed by SQLite and opaque secret material backed
/// exclusively by the injected native secret backend.
pub struct ProviderCredentialStore<B: SecureSecretBackend> {
    storage: Arc<SqliteStorage>,
    secrets: SecureSecretStore<B>,
    cache: RwLock<BTreeMap<CredentialAccount, CredentialStatus>>,
}

impl<B: SecureSecretBackend> ProviderCredentialStore<B> {
    pub fn new(storage: &SqliteStorage, backend: B) -> Self {
        Self {
            storage: Arc::new(storage.clone()),
            secrets: SecureSecretStore::new(backend),
            cache: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn backend_kind(&self) -> BackendKind {
        self.secrets.backend_kind()
    }

    pub fn backend_status(&self) -> BackendStatus {
        self.secrets.backend_status()
    }

    /// Resolves secret material for an opaque reference after reloading its
    /// project/provider/account identity from durable metadata. The secret
    /// itself is never stored in SQLite and is returned only to the transport
    /// boundary for the duration of a request.
    pub fn material_for_reference(
        &self,
        credential_ref: &CredentialRef,
    ) -> Result<SecretMaterial, CredentialServiceError> {
        let reference = credential_ref.as_str().to_string();
        let row = self.database(move |pool| async move {
            sqlx::query(
                "SELECT project_id, provider_id, account_id, state FROM provider_accounts WHERE credential_ref = ? LIMIT 1",
            )
            .bind(reference)
            .fetch_optional(&pool)
            .await
        })?;
        let Some(row) = row else {
            return Err(CredentialServiceError::Missing);
        };
        let state: String = row.try_get("state").map_err(|_| CredentialServiceError::Internal)?;
        match state.as_str() {
            STATE_CONNECTED => {}
            STATE_REVOKED => return Err(CredentialServiceError::Revoked),
            STATE_UNAVAILABLE | STATE_ERROR => return Err(CredentialServiceError::Unavailable),
            _ => return Err(CredentialServiceError::Internal),
        }
        let project_id: String = row.try_get("project_id").map_err(|_| CredentialServiceError::Internal)?;
        let provider_id: String = row.try_get("provider_id").map_err(|_| CredentialServiceError::Internal)?;
        let account_id: String = row.try_get("account_id").map_err(|_| CredentialServiceError::Internal)?;
        let project_id = ProjectScopeId::parse(format!("project_{project_id}"))?;
        let account = CredentialAccount::new(
            project_id.clone(),
            ProviderId::parse(provider_id).map_err(|_| CredentialServiceError::InvalidIdentity)?,
            AccountId::parse(account_id)?,
        )?;
        let context = CredentialAccessContext::new(
            project_id,
            "desktop-webview".into(),
            CancellationToken::new(),
        )?;
        self.secrets
            .get(context, account, credential_ref.clone())
            .map_err(map_secret_error)
    }

    /// Connect an opaque reference when the token exchange is intentionally
    /// fixture-only. Real exchanges must call [`Self::connect_with_material`].
    pub fn connect_with_material(
        &self,
        context: CredentialAccessContext,
        account: CredentialAccount,
        credential_ref: CredentialRef,
        material: SecretMaterial,
    ) -> Result<CredentialStatus, CredentialServiceError> {
        validate_context(&context, &account)?;
        self.secrets
            .put(context.clone(), account.clone(), credential_ref.clone(), material)
            .map_err(map_secret_error)?;
        let status = self.persist_connected(&account, &credential_ref)?;
        self.cache_status(status.clone());
        Ok(status)
    }

    fn persist_connected(
        &self,
        account: &CredentialAccount,
        credential_ref: &CredentialRef,
    ) -> Result<CredentialStatus, CredentialServiceError> {
        let updated_at = chrono::Utc::now().to_rfc3339();
        self.persist_metadata(account, STATE_CONNECTED, Some(credential_ref), &updated_at)?;
        Ok(CredentialStatus {
            account: account.clone(),
            state: CredentialServiceState::Connected,
            credential_ref: Some(credential_ref.clone()),
        })
    }

    fn cache_status(&self, status: CredentialStatus) {
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(status.account.clone(), status);
        }
    }

    fn cached_status(&self, account: &CredentialAccount) -> Option<CredentialStatus> {
        self.cache.read().ok()?.get(account).cloned()
    }

    fn persist_metadata(
        &self,
        account: &CredentialAccount,
        state: &str,
        credential_ref: Option<&CredentialRef>,
        updated_at: &str,
    ) -> Result<(), CredentialServiceError> {
        let project_id = database_project_id(account)?;
        let provider_id = account.provider_id.as_str().to_string();
        let account_id = account.account_id.as_str().to_string();
        let reference = credential_ref.map(|value| value.as_str().to_string());
        let state = state.to_string();
        let updated_at = updated_at.to_string();
        self.database(move |pool| async move {
            sqlx::query(
                "INSERT INTO provider_accounts (project_id, provider_id, account_id, display_name, state, credential_ref, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?) ON CONFLICT(project_id, provider_id, account_id) DO UPDATE SET display_name = excluded.display_name, state = excluded.state, credential_ref = excluded.credential_ref, updated_at = excluded.updated_at",
            )
            .bind(project_id)
            .bind(provider_id)
            .bind(account_id)
            .bind("Provider account")
            .bind(state)
            .bind(reference)
            .bind(updated_at)
            .execute(&pool)
            .await
            .map(|_| ())
        })
    }

    fn load_metadata(
        &self,
        account: &CredentialAccount,
    ) -> Result<Option<MetadataRecord>, CredentialServiceError> {
        let project_id = database_project_id(account)?;
        let provider_id = account.provider_id.as_str().to_string();
        let account_id = account.account_id.as_str().to_string();
        let row = self.database(move |pool| async move {
            sqlx::query(
                "SELECT state, credential_ref, updated_at FROM provider_accounts WHERE project_id = ? AND provider_id = ? AND account_id = ?",
            )
            .bind(project_id)
            .bind(provider_id)
            .bind(account_id)
            .fetch_optional(&pool)
            .await
        })?;
        let Some(row) = row else { return Ok(None) };
        let state: String = row.try_get("state").map_err(|_| CredentialServiceError::Internal)?;
        let state = match state.as_str() {
            STATE_CONNECTED => CredentialServiceState::Connected,
            STATE_REVOKED => CredentialServiceState::Revoked,
            STATE_UNAVAILABLE | STATE_ERROR => CredentialServiceState::Unavailable,
            _ => return Err(CredentialServiceError::Internal),
        };
        let reference: Option<String> = row
            .try_get("credential_ref")
            .map_err(|_| CredentialServiceError::Internal)?;
        let credential_ref = reference
            .map(|value| CredentialRef::parse(value).map_err(|_| CredentialServiceError::InvalidReference))
            .transpose()?;
        Ok(Some(MetadataRecord { state, credential_ref }))
    }

    fn database<T, F, Fut>(&self, operation: F) -> Result<T, CredentialServiceError>
    where
        T: Send + 'static,
        F: FnOnce(Pool<Sqlite>) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, sqlx::Error>> + Send + 'static,
    {
        let pool = self.storage.pool().clone();
        let run = move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| CredentialServiceError::Internal)?;
            runtime
                .block_on(operation(pool))
                .map_err(|_| CredentialServiceError::Internal)
        };
        if tokio::runtime::Handle::try_current().is_ok() {
            std::thread::spawn(run)
                .join()
                .map_err(|_| CredentialServiceError::Internal)?
        } else {
            run()
        }
    }
}

impl<B: SecureSecretBackend> CredentialService for ProviderCredentialStore<B> {
    fn connect(
        &self,
        context: CredentialAccessContext,
        account: CredentialAccount,
        credential_ref: CredentialRef,
    ) -> Result<CredentialStatus, CredentialServiceError> {
        validate_context(&context, &account)?;
        if self.backend_status() != BackendStatus::Available {
            return Err(CredentialServiceError::Unavailable);
        }
        if self.cached_status(&account).is_some_and(|status| status.state == CredentialServiceState::Connected) {
            return Err(CredentialServiceError::Conflict);
        }
        let status = self.persist_connected(&account, &credential_ref)?;
        self.cache_status(status.clone());
        Ok(status)
    }

    fn disconnect(
        &self,
        context: CredentialAccessContext,
        account: CredentialAccount,
    ) -> Result<CredentialStatus, CredentialServiceError> {
        validate_context(&context, &account)?;
        let metadata = self.load_metadata(&account)?
            .or_else(|| self.cached_status(&account).map(|status| MetadataRecord {
                state: status.state,
                credential_ref: status.credential_ref,
            }))
            .ok_or(CredentialServiceError::Missing)?;
        if let Some(reference) = metadata.credential_ref.as_ref() {
            match self.secrets.delete(context.clone(), account.clone(), reference.clone()) {
                Ok(()) | Err(SecretStoreError::Missing) => {}
                Err(error) => return Err(map_secret_error(error)),
            }
        }
        let updated_at = chrono::Utc::now().to_rfc3339();
        self.persist_metadata(&account, STATE_REVOKED, None, &updated_at)?;
        let status = CredentialStatus { account, state: CredentialServiceState::Revoked, credential_ref: None };
        self.cache_status(status.clone());
        Ok(status)
    }

    fn status(
        &self,
        context: CredentialAccessContext,
        account: CredentialAccount,
    ) -> Result<CredentialStatus, CredentialServiceError> {
        validate_context(&context, &account)?;
        if let Some(status) = self.cached_status(&account) {
            return Ok(status);
        }
        let metadata = self.load_metadata(&account)?.ok_or(CredentialServiceError::Missing)?;
        match metadata.state {
            CredentialServiceState::Revoked => Ok(CredentialStatus { account, state: CredentialServiceState::Revoked, credential_ref: None }),
            CredentialServiceState::Unavailable => Ok(CredentialStatus { account, state: CredentialServiceState::Unavailable, credential_ref: None }),
            CredentialServiceState::Connected => {
                let Some(reference) = metadata.credential_ref else {
                    return Ok(CredentialStatus { account, state: CredentialServiceState::Unavailable, credential_ref: None });
                };
                let available = match self.secrets.get(context, account.clone(), reference.clone()) {
                    Ok(_) => true,
                    Err(SecretStoreError::Missing | SecretStoreError::Unavailable) => false,
                    Err(error) => return Err(map_secret_error(error)),
                };
                if available {
                    Ok(CredentialStatus { account, state: CredentialServiceState::Connected, credential_ref: Some(reference) })
                } else {
                    Ok(CredentialStatus { account, state: CredentialServiceState::Unavailable, credential_ref: None })
                }
            }
        }
    }

    fn resolve_ref(
        &self,
        context: CredentialAccessContext,
        account: CredentialAccount,
    ) -> Result<CredentialRef, CredentialServiceError> {
        validate_context(&context, &account)?;
        if let Some(status) = self.cached_status(&account) {
            return status.credential_ref.ok_or(CredentialServiceError::Revoked);
        }
        let status = self.status(context, account)?;
        if status.state != CredentialServiceState::Connected {
            return Err(if status.state == CredentialServiceState::Unavailable {
                CredentialServiceError::Unavailable
            } else {
                CredentialServiceError::Revoked
            });
        }
        status.credential_ref.ok_or(CredentialServiceError::Revoked)
    }
}

fn validate_context(context: &CredentialAccessContext, account: &CredentialAccount) -> Result<(), CredentialServiceError> {
    if context.cancellation.is_cancelled() { return Err(CredentialServiceError::Cancelled); }
    if context.project_id != account.project_id { return Err(CredentialServiceError::Unauthorized); }
    Ok(())
}

fn database_project_id(account: &CredentialAccount) -> Result<String, CredentialServiceError> {
    account
        .project_id
        .as_str()
        .strip_prefix("project_")
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(CredentialServiceError::InvalidIdentity)
}

fn map_secret_error(error: SecretStoreError) -> CredentialServiceError {
    match error {
        SecretStoreError::Unavailable => CredentialServiceError::Unavailable,
        SecretStoreError::Missing => CredentialServiceError::Missing,
        SecretStoreError::Unauthorized => CredentialServiceError::Unauthorized,
        SecretStoreError::Cancelled => CredentialServiceError::Cancelled,
        SecretStoreError::InvalidReference => CredentialServiceError::InvalidReference,
        SecretStoreError::InvalidMaterial | SecretStoreError::Backend => CredentialServiceError::Internal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_runtime::migrations::run_migrations;
    use agent_runtime::sqlite::SqliteStorageConfig;
    use provider_core::credentials::{AccountId, CredentialServiceState, ProjectScopeId};
    use provider_core::{CancellationToken, ProviderId};

    #[tokio::test]
    async fn provider_account_metadata_is_project_scoped_and_cascades() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();
        let sql = sqlx::query_scalar::<_, String>(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'provider_accounts'",
        )
        .fetch_one(storage.pool())
        .await
        .unwrap();
        assert!(sql.contains("PRIMARY KEY (project_id, provider_id, account_id)"));
        assert!(sql.contains("ON DELETE CASCADE"));
    }

    #[tokio::test]
    async fn connected_metadata_is_unavailable_after_restart_without_secret_backend() {
        let path = std::env::temp_dir().join(format!("hank-provider-{}.db", uuid::Uuid::new_v4()));
        let storage = SqliteStorage::connect(SqliteStorageConfig::for_file(&path)).await.unwrap();
        run_migrations(storage.pool()).await.unwrap();
        let project = "proj-00000000-0000-4000-8000-000000000320";
        sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Restart Test', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
            .bind(project)
            .execute(storage.pool())
            .await
            .unwrap();
        let account = CredentialAccount::new(
            ProjectScopeId::parse(format!("project_{project}")).unwrap(),
            ProviderId::parse("mock").unwrap(),
            AccountId::parse("account_restart").unwrap(),
        )
        .unwrap();
        let context = CredentialAccessContext::new(
            account.project_id.clone(),
            "desktop-webview".to_string(),
            CancellationToken::new(),
        )
        .unwrap();
        let reference = CredentialRef::parse("cred_restart").unwrap();
        let first = ProviderCredentialStore::new(&storage, TestBackend::available());
        first.connect(context.clone(), account.clone(), reference).unwrap();
        let second = ProviderCredentialStore::new(&storage, TestBackend::missing());
        let status = second.status(context, account).unwrap();
        assert_eq!(status.state, CredentialServiceState::Unavailable);
        assert!(status.credential_ref.is_none());
        storage.close().await;
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn material_resolution_reloads_identity_and_never_uses_sqlite_plaintext() {
        let path = std::env::temp_dir().join(format!("hank-provider-material-{}.db", uuid::Uuid::new_v4()));
        let storage = SqliteStorage::connect(SqliteStorageConfig::for_file(&path)).await.unwrap();
        run_migrations(storage.pool()).await.unwrap();
        let project = "proj-00000000-0000-4000-8000-000000000322";
        sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Material Test', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
            .bind(project)
            .execute(storage.pool())
            .await
            .unwrap();
        let account = CredentialAccount::new(
            ProjectScopeId::parse(format!("project_{project}")).unwrap(),
            ProviderId::parse("openai").unwrap(),
            AccountId::parse("account_material").unwrap(),
        )
        .unwrap();
        let context = CredentialAccessContext::new(
            account.project_id.clone(),
            "desktop-webview".to_string(),
            CancellationToken::new(),
        )
        .unwrap();
        let reference = CredentialRef::parse("cred_material_test").unwrap();
        let backend = MaterialBackend::default();
        let store = ProviderCredentialStore::new(&storage, backend);
        store
            .connect_with_material(
                context,
                account,
                reference.clone(),
                SecretMaterial::new(b"material-never-in-sqlite".to_vec()).unwrap(),
            )
            .unwrap();
        let material = store.material_for_reference(&reference).unwrap();
        assert_eq!(material.as_bytes(), b"material-never-in-sqlite");
        let rows = sqlx::query_as::<_, (Option<String>,)>(
            "SELECT credential_ref FROM provider_accounts WHERE credential_ref = 'cred_material_test'",
        )
        .fetch_all(storage.pool())
        .await
        .unwrap();
        assert_eq!(rows, vec![(Some("cred_material_test".into()),)]);
        let table_dump: String = sqlx::query_scalar("SELECT quote(display_name) || quote(state) FROM provider_accounts")
            .fetch_one(storage.pool())
            .await
            .unwrap();
        assert!(!table_dump.contains("material-never-in-sqlite"));
        storage.close().await;
        let _ = std::fs::remove_file(path);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn native_backend_roundtrip_persists_and_disconnects_secret_and_metadata() {
        let path = std::env::temp_dir().join(format!("hank-provider-{}.db", uuid::Uuid::new_v4()));
        let storage = SqliteStorage::connect(SqliteStorageConfig::for_file(&path)).await.unwrap();
        run_migrations(storage.pool()).await.unwrap();
        let project = "proj-00000000-0000-4000-8000-000000000321";
        sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Credential Test', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
            .bind(project)
            .execute(storage.pool())
            .await
            .unwrap();
        let account = CredentialAccount::new(
            ProjectScopeId::parse(format!("project_{project}")).unwrap(),
            ProviderId::parse("mock").unwrap(),
            AccountId::parse("account_test").unwrap(),
        )
        .unwrap();
        let context = CredentialAccessContext::new(
            account.project_id.clone(),
            "desktop-webview".to_string(),
            CancellationToken::new(),
        )
        .unwrap();
        let reference = CredentialRef::parse("cred_native_test").unwrap();
        let store = ProviderCredentialStore::new(&storage, crate::platform_store::PlatformSecretBackend);
        store.connect_with_material(context.clone(), account.clone(), reference.clone(), SecretMaterial::new(b"synthetic-test-material".to_vec()).unwrap()).unwrap();
        let recreated = ProviderCredentialStore::new(&storage, crate::platform_store::PlatformSecretBackend);
        assert_eq!(recreated.status(context.clone(), account.clone()).unwrap().state, CredentialServiceState::Connected);
        assert_eq!(recreated.resolve_ref(context.clone(), account.clone()).unwrap(), reference);
        recreated.disconnect(context.clone(), account.clone()).unwrap();
        assert_eq!(recreated.status(context, account).unwrap().state, CredentialServiceState::Revoked);
        storage.close().await;
        let _ = std::fs::remove_file(path);
    }

    #[derive(Clone, Copy)]
    struct TestBackend {
        state: BackendStatus,
    }

    impl TestBackend {
        fn available() -> Self { Self { state: BackendStatus::Available } }
        fn missing() -> Self { Self { state: BackendStatus::Available } }
    }

    impl SecureSecretBackend for TestBackend {
        fn kind(&self) -> BackendKind { BackendKind::Mock }
        fn status(&self) -> BackendStatus { self.state }
        fn put(&self, _: &CredentialRef, _: &CredentialAccount, _: SecretMaterial) -> Result<(), SecretStoreError> { Ok(()) }
        fn get(&self, _: &CredentialRef, _: &CredentialAccount) -> Result<SecretMaterial, SecretStoreError> { Err(SecretStoreError::Missing) }
        fn delete(&self, _: &CredentialRef, _: &CredentialAccount) -> Result<(), SecretStoreError> { Ok(()) }
        fn rotate(&self, _: &CredentialRef, _: &CredentialAccount, _: SecretMaterial) -> Result<(), SecretStoreError> { Ok(()) }
    }

    #[derive(Clone, Default)]
    struct MaterialBackend {
        material: Arc<std::sync::Mutex<Option<Vec<u8>>>>,
    }

    impl SecureSecretBackend for MaterialBackend {
        fn kind(&self) -> BackendKind { BackendKind::Mock }
        fn status(&self) -> BackendStatus { BackendStatus::Available }
        fn put(&self, _: &CredentialRef, _: &CredentialAccount, material: SecretMaterial) -> Result<(), SecretStoreError> {
            *self.material.lock().unwrap() = Some(material.into_bytes());
            Ok(())
        }
        fn get(&self, _: &CredentialRef, _: &CredentialAccount) -> Result<SecretMaterial, SecretStoreError> {
            self.material.lock().unwrap().clone().map(SecretMaterial::new).transpose()?.ok_or(SecretStoreError::Missing)
        }
        fn delete(&self, _: &CredentialRef, _: &CredentialAccount) -> Result<(), SecretStoreError> {
            *self.material.lock().unwrap() = None;
            Ok(())
        }
        fn rotate(&self, reference: &CredentialRef, account: &CredentialAccount, material: SecretMaterial) -> Result<(), SecretStoreError> {
            self.put(reference, account, material)
        }
    }
}
