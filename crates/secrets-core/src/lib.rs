//! Secure secret storage boundary.
//!
//! This crate does not implement plaintext persistence. It delegates secret
//! bytes to an injected OS keychain/Stronghold backend and fails closed when
//! that backend is unavailable. The backend interface is the platform seam.

use provider_core::credentials::{CredentialAccessContext, CredentialAccount};
use provider_core::CredentialRef;
use std::fmt;
use thiserror::Error;

pub const MAX_SECRET_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    OsKeychain,
    TauriStronghold,
    Mock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendStatus {
    Available,
    Unavailable,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretMaterial(Vec<u8>);

impl SecretMaterial {
    pub fn new(bytes: Vec<u8>) -> Result<Self, SecretStoreError> {
        if bytes.is_empty() || bytes.len() > MAX_SECRET_BYTES {
            return Err(SecretStoreError::InvalidMaterial);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn into_bytes(mut self) -> Vec<u8> {
        std::mem::take(&mut self.0)
    }
}

impl fmt::Debug for SecretMaterial {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SecretMaterial")
            .field("bytes", &"[REDACTED]")
            .field("length", &self.0.len())
            .finish()
    }
}

impl Drop for SecretMaterial {
    fn drop(&mut self) {
        for byte in &mut self.0 {
            *byte = 0;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SecretStoreError {
    #[error("secret store backend is unavailable")]
    Unavailable,
    #[error("secret record is missing")]
    Missing,
    #[error("secret access is unauthorized")]
    Unauthorized,
    #[error("secret material is invalid or exceeds bounds")]
    InvalidMaterial,
    #[error("secret operation was cancelled")]
    Cancelled,
    #[error("secret reference is invalid")]
    InvalidReference,
    #[error("secret backend operation failed")]
    Backend,
}

pub trait SecureSecretBackend: Send + Sync {
    fn kind(&self) -> BackendKind;
    fn status(&self) -> BackendStatus;
    fn put(
        &self,
        reference: &CredentialRef,
        account: &CredentialAccount,
        material: SecretMaterial,
    ) -> Result<(), SecretStoreError>;
    fn get(
        &self,
        reference: &CredentialRef,
        account: &CredentialAccount,
    ) -> Result<SecretMaterial, SecretStoreError>;
    fn delete(
        &self,
        reference: &CredentialRef,
        account: &CredentialAccount,
    ) -> Result<(), SecretStoreError>;
    fn rotate(
        &self,
        reference: &CredentialRef,
        account: &CredentialAccount,
        material: SecretMaterial,
    ) -> Result<(), SecretStoreError>;
}

pub struct SecureSecretStore<B> {
    backend: B,
}

impl<B: SecureSecretBackend> SecureSecretStore<B> {
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    pub fn backend_kind(&self) -> BackendKind {
        self.backend.kind()
    }

    pub fn backend_status(&self) -> BackendStatus {
        self.backend.status()
    }

    pub fn put(
        &self,
        context: CredentialAccessContext,
        account: CredentialAccount,
        reference: CredentialRef,
        material: SecretMaterial,
    ) -> Result<(), SecretStoreError> {
        validate_context(&context, &account)?;
        ensure_available(&self.backend)?;
        self.backend.put(&reference, &account, material)
    }

    pub fn get(
        &self,
        context: CredentialAccessContext,
        account: CredentialAccount,
        reference: CredentialRef,
    ) -> Result<SecretMaterial, SecretStoreError> {
        validate_context(&context, &account)?;
        ensure_available(&self.backend)?;
        self.backend.get(&reference, &account)
    }

    pub fn delete(
        &self,
        context: CredentialAccessContext,
        account: CredentialAccount,
        reference: CredentialRef,
    ) -> Result<(), SecretStoreError> {
        validate_context(&context, &account)?;
        ensure_available(&self.backend)?;
        self.backend.delete(&reference, &account)
    }

    pub fn rotate(
        &self,
        context: CredentialAccessContext,
        account: CredentialAccount,
        reference: CredentialRef,
        material: SecretMaterial,
    ) -> Result<(), SecretStoreError> {
        validate_context(&context, &account)?;
        ensure_available(&self.backend)?;
        self.backend.rotate(&reference, &account, material)
    }
}

fn validate_context(
    context: &CredentialAccessContext,
    account: &CredentialAccount,
) -> Result<(), SecretStoreError> {
    if context.cancellation.is_cancelled() {
        return Err(SecretStoreError::Cancelled);
    }
    if context.project_id != account.project_id {
        return Err(SecretStoreError::Unauthorized);
    }
    Ok(())
}

fn ensure_available<B: SecureSecretBackend>(backend: &B) -> Result<(), SecretStoreError> {
    if backend.status() == BackendStatus::Available {
        Ok(())
    } else {
        Err(SecretStoreError::Unavailable)
    }
}
