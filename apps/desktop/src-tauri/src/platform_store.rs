//! Native secure-secret backend for the desktop shell.
//!
//! Windows uses the user-scoped Credential Manager. Other platforms remain
//! explicitly unavailable until their native keychain adapters are added;
//! there is intentionally no plaintext-file or SQLite fallback. The explicit
//! `HANK_E2E_MOCK_PROVIDER=1` fixture uses a process-local, zeroized map so
//! deterministic native E2E can exercise the credential lifecycle without
//! weakening the production boundary.

use provider_core::credentials::CredentialAccount;
use provider_core::CredentialRef;
use secrets_core::{
    BackendKind, BackendStatus, SecretMaterial, SecretStoreError, SecureSecretBackend,
};
#[cfg(not(windows))]
use std::collections::BTreeMap;
#[cfg(not(windows))]
use std::sync::{Mutex, OnceLock};

const TARGET_PREFIX: &str = "Hank/credential/v1";
const MOCK_ENV: &str = "HANK_E2E_MOCK_PROVIDER";
#[cfg(not(windows))]
static FIXTURE_MATERIAL: OnceLock<Mutex<BTreeMap<String, Vec<u8>>>> = OnceLock::new();

#[derive(Debug, Clone, Copy, Default)]
pub struct PlatformSecretBackend;

impl PlatformSecretBackend {
    fn fixture_enabled() -> bool {
        std::env::var(MOCK_ENV).ok().as_deref() == Some("1")
    }

    fn fixture_key(
        reference: &CredentialRef,
        account: &CredentialAccount,
    ) -> Result<String, SecretStoreError> {
        let target = format!(
            "{TARGET_PREFIX}/{}/{}/{}/{}",
            account.project_id.as_str(),
            account.provider_id.as_str(),
            account.account_id.as_str(),
            reference.as_str()
        );
        if target.len() > 512 || target.chars().any(char::is_control) {
            return Err(SecretStoreError::InvalidReference);
        }
        Ok(target)
    }

    fn target_name(
        reference: &CredentialRef,
        account: &CredentialAccount,
    ) -> Result<Vec<u16>, SecretStoreError> {
        let target = Self::fixture_key(reference, account)?;
        Ok(target.encode_utf16().chain(std::iter::once(0)).collect())
    }

    #[cfg(not(windows))]
    fn fixture_material() -> &'static Mutex<BTreeMap<String, Vec<u8>>> {
        FIXTURE_MATERIAL.get_or_init(|| Mutex::new(BTreeMap::new()))
    }
}

impl SecureSecretBackend for PlatformSecretBackend {
    fn kind(&self) -> BackendKind {
        if cfg!(not(windows)) && Self::fixture_enabled() {
            BackendKind::Mock
        } else {
            BackendKind::OsKeychain
        }
    }

    fn status(&self) -> BackendStatus {
        if cfg!(windows) || Self::fixture_enabled() {
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
        #[cfg(windows)]
        {
            use windows_sys::Win32::Security::Credentials::{
                CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
            };

            let mut target = Self::target_name(reference, account)?;
            let mut bytes = material.into_bytes();
            if bytes.len() > 2_560 {
                wipe(&mut bytes);
                return Err(SecretStoreError::InvalidMaterial);
            }
            let credential = CREDENTIALW {
                Type: CRED_TYPE_GENERIC,
                TargetName: target.as_mut_ptr(),
                CredentialBlobSize: bytes.len() as u32,
                CredentialBlob: bytes.as_mut_ptr(),
                Persist: CRED_PERSIST_LOCAL_MACHINE,
                ..Default::default()
            };
            let result = unsafe { CredWriteW(&credential, 0) };
            wipe(&mut bytes);
            if result == 0 {
                return Err(SecretStoreError::Backend);
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            if !Self::fixture_enabled() {
                return Err(SecretStoreError::Unavailable);
            }
            let key = Self::fixture_key(reference, account)?;
            let mut bytes = material.into_bytes();
            let mut records = Self::fixture_material()
                .lock()
                .map_err(|_| SecretStoreError::Backend)?;
            if let Some(mut previous) = records.insert(key, std::mem::take(&mut bytes)) {
                wipe(&mut previous);
            }
            wipe(&mut bytes);
            Ok(())
        }
    }

    fn get(
        &self,
        reference: &CredentialRef,
        account: &CredentialAccount,
    ) -> Result<SecretMaterial, SecretStoreError> {
        #[cfg(windows)]
        {
            use std::ptr::null_mut;
            use windows_sys::Win32::Security::Credentials::{
                CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
            };

            let target = Self::target_name(reference, account)?;
            let mut raw: *mut CREDENTIALW = null_mut();
            let result = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut raw) };
            if result == 0 {
                return if std::io::Error::last_os_error().raw_os_error() == Some(1168) {
                    Err(SecretStoreError::Missing)
                } else {
                    Err(SecretStoreError::Backend)
                };
            }
            let material = unsafe {
                if raw.is_null() {
                    Err(SecretStoreError::Backend)
                } else {
                    let credential = &*raw;
                    let size = credential.CredentialBlobSize as usize;
                    if credential.CredentialBlob.is_null() || size == 0 || size > 2_560 {
                        Err(SecretStoreError::Backend)
                    } else {
                        let bytes =
                            std::slice::from_raw_parts(credential.CredentialBlob, size).to_vec();
                        SecretMaterial::new(bytes)
                    }
                }
            };
            unsafe { CredFree(raw.cast()) };
            material
        }
        #[cfg(not(windows))]
        {
            if !Self::fixture_enabled() {
                return Err(SecretStoreError::Unavailable);
            }
            let key = Self::fixture_key(reference, account)?;
            let records = Self::fixture_material()
                .lock()
                .map_err(|_| SecretStoreError::Backend)?;
            let bytes = records.get(&key).ok_or(SecretStoreError::Missing)?.clone();
            SecretMaterial::new(bytes)
        }
    }

    fn delete(
        &self,
        reference: &CredentialRef,
        account: &CredentialAccount,
    ) -> Result<(), SecretStoreError> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC};

            let target = Self::target_name(reference, account)?;
            let result = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
            if result == 0 {
                return if std::io::Error::last_os_error().raw_os_error() == Some(1168) {
                    Err(SecretStoreError::Missing)
                } else {
                    Err(SecretStoreError::Backend)
                };
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            if !Self::fixture_enabled() {
                return Err(SecretStoreError::Unavailable);
            }
            let key = Self::fixture_key(reference, account)?;
            let mut records = Self::fixture_material()
                .lock()
                .map_err(|_| SecretStoreError::Backend)?;
            let Some(mut bytes) = records.remove(&key) else {
                return Err(SecretStoreError::Missing);
            };
            wipe(&mut bytes);
            Ok(())
        }
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

fn wipe(bytes: &mut [u8]) {
    for byte in bytes {
        *byte = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use provider_core::credentials::{AccountId, ProjectScopeId};
    use provider_core::ProviderId;

    fn account() -> CredentialAccount {
        CredentialAccount::new(
            ProjectScopeId::parse("project_test").unwrap(),
            ProviderId::parse("mock").unwrap(),
            AccountId::parse("account_test").unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn target_name_contains_only_bounded_opaque_identity() {
        let reference = CredentialRef::parse("cred_test").unwrap();
        let target = PlatformSecretBackend::target_name(&reference, &account()).unwrap();
        let text = String::from_utf16(&target[..target.len() - 1]).unwrap();
        assert_eq!(
            text,
            "Hank/credential/v1/project_test/mock/account_test/cred_test"
        );
        assert!(!text.contains("secret"));
    }

    #[test]
    fn non_windows_backend_is_explicitly_unavailable() {
        if !cfg!(windows) {
            assert_eq!(PlatformSecretBackend.status(), BackendStatus::Unavailable);
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_credential_manager_roundtrip_is_scoped_and_cleaned_up() {
        let backend = PlatformSecretBackend;
        let account = account();
        let reference = CredentialRef::parse("cred_native_test").unwrap();
        let material = SecretMaterial::new(b"synthetic-test-material".to_vec()).unwrap();

        backend
            .put(&reference, &account, material)
            .expect("Credential Manager should accept the bounded test material");
        let loaded = backend
            .get(&reference, &account)
            .expect("Credential Manager should return the scoped test material");
        assert_eq!(loaded.as_bytes(), b"synthetic-test-material");

        backend
            .delete(&reference, &account)
            .expect("Credential Manager cleanup should succeed");
        assert!(matches!(
            backend.get(&reference, &account),
            Err(SecretStoreError::Missing)
        ));
    }
}
