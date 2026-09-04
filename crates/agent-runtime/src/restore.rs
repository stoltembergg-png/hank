//! Fail-closed, staged restore for verified SQLite profile backups.
//!
//! This module owns the bounded filesystem part of restore. It verifies a
//! backup through [`DatabaseBackupService`], migrates only an isolated stage,
//! and promotes it under an exclusive restore lock. The application boundary
//! supplies the opaque operator authorization; no secret material is accepted.

use crate::backup::{BackupVerification, DatabaseBackupService, VerificationError};
use crate::migrations::run_migrations;
use crate::sqlite::{SqliteError, SqliteStorage, SqliteStorageConfig};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Sqlite};
use std::path::{Component, Path, PathBuf};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const RESTORE_FORMAT_VERSION: u16 = 1;

const MAX_ID_BYTES: usize = 128;
const MAX_METADATA_BYTES: usize = 256;
const MAX_RECEIPT_BYTES: u64 = 16 * 1024;

/// Allowlist and resource limits for one restore boundary.
#[derive(Debug, Clone)]
pub struct RestorePolicy {
    pub target_root: PathBuf,
    pub max_restore_bytes: u64,
    pub current_schema_version: i64,
}

impl RestorePolicy {
    pub fn new(
        target_root: impl Into<PathBuf>,
        max_restore_bytes: u64,
        current_schema_version: i64,
    ) -> Result<Self, RestoreError> {
        let target_root = target_root.into();
        if target_root.as_os_str().is_empty()
            || max_restore_bytes == 0
            || current_schema_version < 0
            || !is_safe_path(&target_root)
        {
            return Err(RestoreError::InvalidPolicy);
        }
        Ok(Self {
            target_root,
            max_restore_bytes,
            current_schema_version,
        })
    }
}

/// Opaque authorization supplied by the approved application boundary.
#[derive(Debug, Clone)]
pub struct RestoreAuthorization {
    pub actor_id: String,
    pub confirmation_id: String,
    pub request_digest: String,
}

/// Explicit, bounded restore command.
#[derive(Debug, Clone)]
pub struct RestoreRequest {
    pub restore_id: String,
    pub source_manifest_path: PathBuf,
    pub source_backup_id: String,
    pub target_profile_id: String,
    pub target_database_path: PathBuf,
    pub target_schema_version: i64,
    pub authorization: RestoreAuthorization,
    pub dry_run: bool,
}

impl RestoreRequest {
    fn validate(&self) -> Result<(), RestoreError> {
        if !valid_identifier(&self.restore_id)
            || !valid_text(&self.source_backup_id, MAX_METADATA_BYTES)
            || !valid_text(&self.target_profile_id, MAX_METADATA_BYTES)
            || !self.target_database_path.is_absolute()
            || !is_safe_path(&self.target_database_path)
            || self.target_schema_version < 0
            || !valid_text(&self.authorization.actor_id, MAX_METADATA_BYTES)
            || !valid_text(&self.authorization.confirmation_id, MAX_METADATA_BYTES)
            || !is_sha256(&self.authorization.request_digest)
            || self.authorization.request_digest != restore_request_digest(self)
        {
            return Err(RestoreError::InvalidRequest);
        }
        Ok(())
    }
}

/// Stable digest bound to the source, target, schema and operator intent.
///
/// The digest is not an authorization primitive by itself. It prevents an
/// opaque authorization from being replayed against a different request.
pub fn restore_request_digest(request: &RestoreRequest) -> String {
    let material = format!(
        "restore-v{RESTORE_FORMAT_VERSION}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        request.restore_id,
        request.source_manifest_path.to_string_lossy(),
        request.source_backup_id,
        request.target_profile_id,
        request.target_database_path.to_string_lossy(),
        request.target_schema_version,
        request.dry_run,
        request.authorization.actor_id,
        request.authorization.confirmation_id,
    );
    digest_bytes(material.as_bytes())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreOutcome {
    DryRun,
    Applied,
    AlreadyApplied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreResult {
    pub outcome: RestoreOutcome,
    pub restore_id: String,
    pub source_backup_id: String,
    pub target_profile_id: String,
    pub schema_version: i64,
    pub requires_migration: bool,
    pub compatible: bool,
    pub target_sha256: String,
}

#[derive(Debug, Error)]
pub enum RestoreError {
    #[error("invalid restore policy")]
    InvalidPolicy,
    #[error("invalid restore request")]
    InvalidRequest,
    #[error("restore source verification failed")]
    SourceVerification(#[source] VerificationError),
    #[error("restore source backup identity does not match the request")]
    SourceBackupMismatch,
    #[error("restore source profile does not match the target profile")]
    ProfileMismatch,
    #[error("restore schema is incompatible with the explicit target")]
    IncompatibleSchema {
        source_schema_version: i64,
        target_schema_version: i64,
    },
    #[error("restore target is outside the configured root")]
    TargetOutsideRoot,
    #[error("restore target is a symlink")]
    TargetSymlink,
    #[error("restore target is invalid")]
    TargetInvalid,
    #[error("restore source and target are the same file")]
    SourceTargetConflict,
    #[error("restore target is locked")]
    TargetLocked,
    #[error("restore request conflicts with existing restore state")]
    RestoreConflict,
    #[error("restore artifact exceeds the configured limit")]
    TooLarge,
    #[error("restore storage could not be opened")]
    Storage(#[source] SqliteError),
    #[error("restore migration failed")]
    Migration(#[source] SqliteError),
    #[error("restore database validation failed")]
    Database(#[source] sqlx::Error),
    #[error("restore I/O failed")]
    Io(#[source] std::io::Error),
    #[error("restore receipt is invalid")]
    InvalidReceipt,
    #[error("restore receipt serialization failed")]
    Serialization(#[source] serde_json::Error),
    #[error("restore promotion failed")]
    Promotion(#[source] std::io::Error),
    #[error("restore rollback failed")]
    Rollback(#[source] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RestoreReceipt {
    format_version: u16,
    restore_id: String,
    request_digest: String,
    actor_id: String,
    source_backup_id: String,
    source_manifest_sha256: String,
    target_profile_id: String,
    schema_version: i64,
    target_sha256: String,
}

struct RestorePaths {
    target: PathBuf,
    lock: PathBuf,
    stage: PathBuf,
    previous: PathBuf,
    receipt: PathBuf,
}

struct RestoreLock {
    path: PathBuf,
}

impl Drop for RestoreLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

impl RestoreLock {
    async fn acquire(path: PathBuf) -> Result<Self, RestoreError> {
        let mut file = match tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(RestoreError::TargetLocked);
            }
            Err(error) => return Err(RestoreError::Io(error)),
        };
        let result = async {
            file.write_all(b"hank-restore-lock-v1")
                .await
                .map_err(RestoreError::Io)?;
            file.sync_all().await.map_err(RestoreError::Io)
        }
        .await;
        if result.is_err() {
            let _ = tokio::fs::remove_file(&path).await;
        }
        result.map(|()| Self { path })
    }
}

/// Stages and promotes one already-verifiable backup without touching an
/// unrelated profile path.
#[derive(Clone)]
pub struct DatabaseRestoreService {
    backup: DatabaseBackupService,
    policy: RestorePolicy,
}

impl DatabaseRestoreService {
    pub fn new(backup: DatabaseBackupService, policy: RestorePolicy) -> Self {
        Self { backup, policy }
    }

    /// Verify, optionally plan, and atomically promote a staged profile.
    pub async fn restore(&self, request: RestoreRequest) -> Result<RestoreResult, RestoreError> {
        request.validate()?;
        let verified = self
            .backup
            .verify(&request.source_manifest_path)
            .await
            .map_err(RestoreError::SourceVerification)?;
        if verified.manifest.backup_id != request.source_backup_id {
            return Err(RestoreError::SourceBackupMismatch);
        }
        if verified.manifest.profile_id != request.target_profile_id {
            return Err(RestoreError::ProfileMismatch);
        }

        let requires_migration = verified.manifest.schema_version < request.target_schema_version;
        let compatible = verified.manifest.schema_version <= request.target_schema_version
            && request.target_schema_version == self.policy.current_schema_version;
        let plan = RestoreResult {
            outcome: RestoreOutcome::DryRun,
            restore_id: request.restore_id.clone(),
            source_backup_id: request.source_backup_id.clone(),
            target_profile_id: request.target_profile_id.clone(),
            schema_version: verified.manifest.schema_version,
            requires_migration,
            compatible,
            target_sha256: String::new(),
        };
        let paths = self.resolve_target(&request.target_database_path).await?;
        if same_file(&verified.database_path, &paths.target).await {
            return Err(RestoreError::SourceTargetConflict);
        }

        if let Some(receipt) = read_receipt(&paths.receipt).await? {
            return self
                .result_from_receipt(&request, &verified, &paths, receipt)
                .await;
        }
        if request.dry_run {
            return Ok(plan);
        }
        if !compatible {
            return Err(RestoreError::IncompatibleSchema {
                source_schema_version: verified.manifest.schema_version,
                target_schema_version: request.target_schema_version,
            });
        }

        let _lock = RestoreLock::acquire(paths.lock.clone()).await?;
        if let Some(receipt) = read_receipt(&paths.receipt).await? {
            return self
                .result_from_receipt(&request, &verified, &paths, receipt)
                .await;
        }
        self.apply(&request, &verified, plan, paths).await
    }

    async fn apply(
        &self,
        request: &RestoreRequest,
        verified: &BackupVerification,
        plan: RestoreResult,
        paths: RestorePaths,
    ) -> Result<RestoreResult, RestoreError> {
        if tokio::fs::try_exists(&paths.stage)
            .await
            .map_err(RestoreError::Io)?
            || tokio::fs::try_exists(&paths.previous)
                .await
                .map_err(RestoreError::Io)?
        {
            return Err(RestoreError::RestoreConflict);
        }

        if let Err(error) = copy_bounded(
            &verified.database_path,
            &paths.stage,
            self.policy.max_restore_bytes,
        )
        .await
        {
            cleanup_stage(&paths).await;
            return Err(error);
        }
        let staged = self
            .prepare_stage(&paths.stage, request.target_schema_version)
            .await;
        let (schema_version, target_sha256) = match staged {
            Ok(value) => value,
            Err(error) => {
                cleanup_stage(&paths).await;
                return Err(error);
            }
        };
        let receipt = RestoreReceipt {
            format_version: RESTORE_FORMAT_VERSION,
            restore_id: request.restore_id.clone(),
            request_digest: request.authorization.request_digest.clone(),
            actor_id: request.authorization.actor_id.clone(),
            source_backup_id: verified.manifest.backup_id.clone(),
            source_manifest_sha256: verified.manifest_sha256.clone(),
            target_profile_id: request.target_profile_id.clone(),
            schema_version,
            target_sha256: target_sha256.clone(),
        };
        let receipt_stage = receipt_stage_path(&paths.receipt);
        if let Err(error) = write_receipt(&receipt_stage, &receipt).await {
            cleanup_stage(&paths).await;
            return Err(error);
        }

        let target_exists = match target_is_regular_file(&paths.target).await {
            Ok(value) => value,
            Err(error) => {
                cleanup_stage(&paths).await;
                let _ = tokio::fs::remove_file(&receipt_stage).await;
                return Err(error);
            }
        };
        if target_exists {
            if let Err(error) = tokio::fs::rename(&paths.target, &paths.previous).await {
                cleanup_stage(&paths).await;
                let _ = tokio::fs::remove_file(&receipt_stage).await;
                return Err(RestoreError::Promotion(error));
            }
        }
        if let Err(error) = tokio::fs::rename(&paths.stage, &paths.target).await {
            let rollback_result = if target_exists {
                rollback_previous(&paths).await
            } else {
                Ok(())
            };
            cleanup_stage(&paths).await;
            let _ = tokio::fs::remove_file(&receipt_stage).await;
            rollback_result?;
            return Err(RestoreError::Promotion(error));
        }
        if let Err(error) = tokio::fs::rename(&receipt_stage, &paths.receipt).await {
            let rollback_result = rollback_promoted(&paths, target_exists).await;
            let _ = tokio::fs::remove_file(&receipt_stage).await;
            rollback_result?;
            return Err(RestoreError::Io(error));
        }
        if target_exists {
            let _ = tokio::fs::remove_file(&paths.previous).await;
        }

        Ok(RestoreResult {
            outcome: RestoreOutcome::Applied,
            schema_version,
            target_sha256,
            ..plan
        })
    }

    async fn prepare_stage(
        &self,
        stage: &Path,
        target_schema_version: i64,
    ) -> Result<(i64, String), RestoreError> {
        let storage = SqliteStorage::connect(SqliteStorageConfig {
            database_path: Some(stage.to_path_buf()),
            max_connections: 1,
            busy_timeout: std::time::Duration::from_secs(5),
            create_if_missing: false,
            wal_mode: false,
            foreign_keys: true,
        })
        .await
        .map_err(RestoreError::Storage)?;
        if let Err(error) = run_migrations(storage.pool()).await {
            storage.close().await;
            return Err(RestoreError::Migration(error));
        }
        let schema = match schema_version(storage.pool()).await {
            Ok(value) => value,
            Err(error) => {
                storage.close().await;
                return Err(RestoreError::Database(error));
            }
        };
        let integrity = match sqlx::query_scalar::<Sqlite, String>("PRAGMA integrity_check")
            .fetch_one(storage.pool())
            .await
        {
            Ok(value) => value,
            Err(error) => {
                storage.close().await;
                return Err(RestoreError::Database(error));
            }
        };
        storage.close().await;
        if integrity != "ok" || schema > target_schema_version {
            return Err(RestoreError::IncompatibleSchema {
                source_schema_version: schema,
                target_schema_version,
            });
        }
        let (_, digest) = hash_file(stage, self.policy.max_restore_bytes).await?;
        Ok((schema, digest))
    }

    async fn result_from_receipt(
        &self,
        request: &RestoreRequest,
        verified: &BackupVerification,
        paths: &RestorePaths,
        receipt: RestoreReceipt,
    ) -> Result<RestoreResult, RestoreError> {
        if receipt.format_version != RESTORE_FORMAT_VERSION
            || receipt.restore_id != request.restore_id
            || receipt.request_digest != request.authorization.request_digest
            || receipt.actor_id != request.authorization.actor_id
            || receipt.source_backup_id != verified.manifest.backup_id
            || receipt.source_manifest_sha256 != verified.manifest_sha256
            || receipt.target_profile_id != request.target_profile_id
        {
            return Err(RestoreError::RestoreConflict);
        }
        let (_, target_sha256) = hash_file(&paths.target, self.policy.max_restore_bytes).await?;
        if target_sha256 != receipt.target_sha256 {
            return Err(RestoreError::RestoreConflict);
        }
        Ok(RestoreResult {
            outcome: RestoreOutcome::AlreadyApplied,
            restore_id: request.restore_id.clone(),
            source_backup_id: request.source_backup_id.clone(),
            target_profile_id: request.target_profile_id.clone(),
            schema_version: receipt.schema_version,
            requires_migration: verified.manifest.schema_version < request.target_schema_version,
            compatible: true,
            target_sha256: receipt.target_sha256,
        })
    }

    async fn resolve_target(&self, target: &Path) -> Result<RestorePaths, RestoreError> {
        let root = self.canonical_root().await?;
        let filename = target
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or(RestoreError::TargetInvalid)?;
        if !is_database_filename(filename) || is_reserved_restore_filename(filename) {
            return Err(RestoreError::TargetInvalid);
        }
        let parent = target.parent().ok_or(RestoreError::TargetOutsideRoot)?;
        let canonical_parent = tokio::fs::canonicalize(parent)
            .await
            .map_err(|_| RestoreError::TargetOutsideRoot)?;
        if canonical_parent != root {
            return Err(RestoreError::TargetOutsideRoot);
        }
        match tokio::fs::symlink_metadata(target).await {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(RestoreError::TargetSymlink);
            }
            Ok(metadata) if !metadata.is_file() => return Err(RestoreError::TargetInvalid),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(RestoreError::Io(error)),
        }
        let target = canonical_parent.join(filename);
        Ok(RestorePaths {
            lock: target.with_file_name(format!("{filename}.restore.lock")),
            stage: target.with_file_name(format!(".{filename}.restore-stage.tmp")),
            previous: target.with_file_name(format!(".{filename}.restore-previous.db")),
            receipt: target.with_file_name(format!(".{filename}.restore-receipt.json")),
            target,
        })
    }

    async fn canonical_root(&self) -> Result<PathBuf, RestoreError> {
        if let Ok(metadata) = tokio::fs::symlink_metadata(&self.policy.target_root).await {
            if metadata.file_type().is_symlink() {
                return Err(RestoreError::TargetOutsideRoot);
            }
        }
        tokio::fs::create_dir_all(&self.policy.target_root)
            .await
            .map_err(RestoreError::Io)?;
        if tokio::fs::symlink_metadata(&self.policy.target_root)
            .await
            .map_err(RestoreError::Io)?
            .file_type()
            .is_symlink()
        {
            return Err(RestoreError::TargetOutsideRoot);
        }
        tokio::fs::canonicalize(&self.policy.target_root)
            .await
            .map_err(RestoreError::Io)
    }
}

/// Return the lock marker path used for an explicit target.
pub fn restore_lock_path(target: impl AsRef<Path>) -> Result<PathBuf, RestoreError> {
    let target = target.as_ref();
    let filename = target
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(RestoreError::TargetInvalid)?;
    if !is_database_filename(filename) || is_reserved_restore_filename(filename) {
        return Err(RestoreError::TargetInvalid);
    }
    Ok(target.with_file_name(format!("{filename}.restore.lock")))
}

async fn read_receipt(path: &Path) -> Result<Option<RestoreReceipt>, RestoreError> {
    let metadata = match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(RestoreError::Io(error)),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(RestoreError::InvalidReceipt);
    }
    if metadata.len() > MAX_RECEIPT_BYTES {
        return Err(RestoreError::InvalidReceipt);
    }
    let bytes = tokio::fs::read(path).await.map_err(RestoreError::Io)?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| RestoreError::InvalidReceipt)
}

async fn write_receipt(path: &Path, receipt: &RestoreReceipt) -> Result<(), RestoreError> {
    let bytes = serde_json::to_vec(receipt).map_err(RestoreError::Serialization)?;
    if bytes.len() as u64 > MAX_RECEIPT_BYTES {
        return Err(RestoreError::InvalidReceipt);
    }
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .await
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                RestoreError::RestoreConflict
            } else {
                RestoreError::Io(error)
            }
        })?;
    let result = async {
        file.write_all(&bytes).await.map_err(RestoreError::Io)?;
        file.sync_all().await.map_err(RestoreError::Io)
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(path).await;
    }
    result
}

fn receipt_stage_path(path: &Path) -> PathBuf {
    path.with_file_name(format!(
        "{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy()
    ))
}

async fn copy_bounded(source: &Path, target: &Path, limit: u64) -> Result<(), RestoreError> {
    let mut source_file = tokio::fs::File::open(source)
        .await
        .map_err(RestoreError::Io)?;
    let mut target_file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .await
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                RestoreError::RestoreConflict
            } else {
                RestoreError::Io(error)
            }
        })?;
    let mut bytes = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = source_file
            .read(&mut buffer)
            .await
            .map_err(RestoreError::Io)?;
        if read == 0 {
            break;
        }
        bytes = bytes.saturating_add(read as u64);
        if bytes > limit {
            return Err(RestoreError::TooLarge);
        }
        target_file
            .write_all(&buffer[..read])
            .await
            .map_err(RestoreError::Io)?;
    }
    target_file.sync_all().await.map_err(RestoreError::Io)
}

async fn hash_file(path: &Path, limit: u64) -> Result<(u64, String), RestoreError> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(RestoreError::Io)?;
    let mut hasher = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).await.map_err(RestoreError::Io)?;
        if read == 0 {
            break;
        }
        bytes = bytes.saturating_add(read as u64);
        if bytes > limit {
            return Err(RestoreError::TooLarge);
        }
        hasher.update(&buffer[..read]);
    }
    Ok((bytes, digest_bytes(hasher.finalize().as_slice())))
}

async fn schema_version(pool: &Pool<Sqlite>) -> Result<i64, sqlx::Error> {
    let has_migrations = sqlx::query_scalar::<Sqlite, i64>(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations')",
    )
    .fetch_one(pool)
    .await?;
    if has_migrations == 0 {
        return Ok(0);
    }
    sqlx::query_scalar::<Sqlite, i64>("SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations")
        .fetch_one(pool)
        .await
}

async fn target_is_regular_file(path: &Path) -> Result<bool, RestoreError> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(RestoreError::TargetSymlink),
        Ok(metadata) if metadata.is_file() => Ok(true),
        Ok(_) => Err(RestoreError::TargetInvalid),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(RestoreError::Io(error)),
    }
}

async fn rollback_previous(paths: &RestorePaths) -> Result<(), RestoreError> {
    tokio::fs::rename(&paths.previous, &paths.target)
        .await
        .map_err(RestoreError::Rollback)
}

async fn rollback_promoted(paths: &RestorePaths, target_exists: bool) -> Result<(), RestoreError> {
    if target_exists {
        let _ = tokio::fs::remove_file(&paths.target).await;
        rollback_previous(paths).await
    } else {
        tokio::fs::remove_file(&paths.target)
            .await
            .map_err(RestoreError::Rollback)
    }
}

async fn cleanup_stage(paths: &RestorePaths) {
    let _ = tokio::fs::remove_file(&paths.stage).await;
}

async fn same_file(left: &Path, right: &Path) -> bool {
    match (
        tokio::fs::canonicalize(left).await,
        tokio::fs::canonicalize(right).await,
    ) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn valid_text(value: &str, max_bytes: usize) -> bool {
    !value.is_empty() && value.len() <= max_bytes && !value.chars().any(char::is_control)
}

fn valid_identifier(value: &str) -> bool {
    valid_text(value, MAX_ID_BYTES)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_database_filename(value: &str) -> bool {
    valid_text(value, MAX_METADATA_BYTES)
        && value.ends_with(".db")
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn is_reserved_restore_filename(value: &str) -> bool {
    value.starts_with('.')
        || value.ends_with(".restore-stage.tmp")
        || value.ends_with(".restore-previous.db")
        || value.ends_with(".restore-receipt.json")
        || value.ends_with(".restore.lock")
}

fn is_safe_path(path: &Path) -> bool {
    !path.to_string_lossy().chars().any(char::is_control)
        && path
            .components()
            .all(|component| !matches!(component, Component::ParentDir))
}

fn digest_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
