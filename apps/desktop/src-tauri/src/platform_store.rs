//! Native secure-secret backend for the desktop shell.
//!
//! Windows uses the user-scoped Credential Manager. Other platforms remain
//! explicitly unavailable until their native keychain adapters are added;
//! there is intentionally no plaintext-file or SQLite fallback.

use provider_core::credentials::CredentialAccount;
use provider_core::CredentialRef;
use secrets_core::{
    BackendKind, BackendStatus, SecretMaterial, SecretStoreError, SecureSecretBackend,
};

const TARGET_PREFIX: &str = "Hank/credential/v1";

#[derive(Debug, Clone, Copy, Default)]
pub struct PlatformSecretBackend;

impl PlatformSecretBackend {
    fn target_name(
        reference: &CredentialRef,
        account: &CredentialAccount,
    ) -> Result<Vec<u16>, SecretStoreError> {
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
        Ok(target.encode_utf16().chain(std::iter::once(0)).collect())
    }
}

impl SecureSecretBackend for PlatformSecretBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::OsKeychain
    }

    fn status(&self) -> BackendStatus {
        if cfg!(windows) {
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
            let _ = (reference, account, material);
            Err(SecretStoreError::Unavailable)
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
                        let bytes = std::slice::from_raw_parts(credential.CredentialBlob, size).to_vec();
                        SecretMaterial::new(bytes)
                    }
                }
            };
            unsafe { CredFree(raw.cast()) };
            material
        }
        #[cfg(not(windows))]
        {
            let _ = (reference, account);
            Err(SecretStoreError::Unavailable)
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
            let _ = (reference, account);
            Err(SecretStoreError::Unavailable)
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
        assert_eq!(text, "Hank/credential/v1/project_test/mock/account_test/cred_test");
        assert!(!text.contains("secret"));
    }

    #[test]
    fn non_windows_backend_is_explicitly_unavailable() {
        if !cfg!(windows) {
            assert_eq!(PlatformSecretBackend.status(), BackendStatus::Unavailable);
        }
    }
}
