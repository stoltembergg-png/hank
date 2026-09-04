//! Bounded, verifiable SQLite profile backups.
//!
//! The backup service owns the durable SQLite adapter boundary. It uses
//! SQLite's online `VACUUM INTO` snapshot, writes only inside an application
//! controlled directory, and publishes a manifest only after the database
//! artifact has been hashed and synced. Restore and remote publication remain
//! outside this module.

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Sqlite;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

use crate::sqlite::{SqliteError, SqliteStorage, SqliteStorageConfig};

pub const BACKUP_FORMAT_VERSION: u16 = 1;
const MAX_MANIFEST_BYTES: u64 = 16 * 1024;
const MAX_METADATA_BYTES: usize = 256;
const MAX_PROFILE_ID_BYTES: usize = 128;
const MAX_SCAN_ENTRIES: usize = 1024;

/// Retention and destination limits for one profile's backups.
#[derive(Debug, Clone)]
pub struct BackupPolicy {
    pub destination_root: PathBuf,
    pub max_backups: usize,
    pub max_total_bytes: u64,
    pub max_backup_bytes: u64,
}

impl BackupPolicy {
    pub fn new(
        destination_root: impl Into<PathBuf>,
        max_backups: usize,
        max_total_bytes: u64,
    ) -> Result<Self, BackupError> {
        let destination_root = destination_root.into();
        if destination_root.as_os_str().is_empty()
            || max_backups == 0
            || max_total_bytes == 0
            || !is_safe_path(&destination_root)
        {
            return Err(BackupError::InvalidPolicy);
        }
        Ok(Self {
            destination_root,
            max_backups,
            max_total_bytes,
            max_backup_bytes: max_total_bytes,
        })
    }

    pub fn with_max_backup_bytes(mut self, max_backup_bytes: u64) -> Result<Self, BackupError> {
        if max_backup_bytes == 0 || max_backup_bytes > self.max_total_bytes {
            return Err(BackupError::InvalidPolicy);
        }
        self.max_backup_bytes = max_backup_bytes;
        Ok(self)
    }
}

/// Protection declaration for the destination directory.
///
/// The service never accepts secret bytes. The reference is an opaque handle
/// owned by the approved OS protection/key-management boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupProtection {
    OsPolicy { key_reference: String },
}

impl BackupProtection {
    fn validate(&self) -> Result<(), BackupError> {
        match self {
            Self::OsPolicy { key_reference } if valid_text(key_reference, MAX_METADATA_BYTES) => {
                Ok(())
            }
            _ => Err(BackupError::InvalidRequest),
        }
    }
}

/// Metadata supplied by the trusted application boundary for one snapshot.
#[derive(Debug, Clone)]
pub struct BackupRequest {
    pub profile_id: String,
    pub app_version: String,
    pub source_revision: String,
    pub source_tree: String,
    pub policy_revision: String,
    pub protection: BackupProtection,
}

impl BackupRequest {
    fn validate(&self) -> Result<(), BackupError> {
        if !valid_text(&self.profile_id, MAX_PROFILE_ID_BYTES)
            || !valid_text(&self.app_version, MAX_METADATA_BYTES)
            || !valid_text(&self.source_revision, MAX_METADATA_BYTES)
            || !valid_text(&self.source_tree, MAX_METADATA_BYTES)
            || !valid_text(&self.policy_revision, MAX_METADATA_BYTES)
        {
            return Err(BackupError::InvalidRequest);
        }
        self.protection.validate()
    }
}

/// JSON metadata paired with a database artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupManifest {
    pub format_version: u16,
    pub backup_id: String,
    pub profile_id: String,
    pub schema_version: i64,
    pub app_version: String,
    pub source_revision: String,
    pub source_tree: String,
    pub policy_revision: String,
    pub protection: BackupProtection,
    pub created_at: String,
    pub database_file: String,
    pub database_size_bytes: u64,
    pub database_sha256: String,
}

impl BackupManifest {
    fn validate(&self) -> Result<(), VerificationError> {
        if self.format_version != BACKUP_FORMAT_VERSION
            || !valid_text(&self.backup_id, MAX_METADATA_BYTES)
            || !valid_text(&self.profile_id, MAX_PROFILE_ID_BYTES)
            || self.schema_version < 0
            || !valid_text(&self.app_version, MAX_METADATA_BYTES)
            || !valid_text(&self.source_revision, MAX_METADATA_BYTES)
            || !valid_text(&self.source_tree, MAX_METADATA_BYTES)
            || !valid_text(&self.policy_revision, MAX_METADATA_BYTES)
            || !valid_text(&self.created_at, MAX_METADATA_BYTES)
            || chrono::DateTime::parse_from_rfc3339(&self.created_at).is_err()
            || !is_database_filename(&self.database_file)
            || !is_sha256(&self.database_sha256)
        {
            return Err(VerificationError::InvalidManifest);
        }
        if !matches!(
            &self.protection,
            BackupProtection::OsPolicy { key_reference }
                if valid_text(key_reference, MAX_METADATA_BYTES)
        ) {
            return Err(VerificationError::InvalidManifest);
        }
        Ok(())
    }
}

/// Result returned after the two artifact files have been atomically published.
#[derive(Debug, Clone)]
pub struct BackupArtifact {
    pub manifest: BackupManifest,
    pub database_path: PathBuf,
    pub manifest_path: PathBuf,
    pub database_size_bytes: u64,
    pub database_sha256: String,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone)]
pub struct BackupVerification {
    pub manifest: BackupManifest,
    pub database_path: PathBuf,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RetentionReport {
    pub retained: Vec<String>,
    pub deleted: Vec<String>,
    pub skipped: usize,
}

#[derive(Debug, Error)]
pub enum BackupError {
    #[error("invalid backup policy")]
    InvalidPolicy,
    #[error("invalid backup request")]
    InvalidRequest,
    #[error("SQLite in-memory databases cannot be backed up")]
    InMemorySource,
    #[error("SQLite source database is unavailable")]
    SourceUnavailable,
    #[error("backup destination is invalid")]
    Destination,
    #[error("backup database operation failed")]
    Database(#[source] sqlx::Error),
    #[error("backup source storage could not be opened")]
    Storage(#[source] SqliteError),
    #[error("backup I/O failed")]
    Io(#[source] std::io::Error),
    #[error("backup manifest serialization failed")]
    Serialization(#[source] serde_json::Error),
    #[error("backup exceeds configured size limit")]
    TooLarge,
    #[error("backup retention failed")]
    Retention(#[source] std::io::Error),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum VerificationError {
    #[error("backup manifest is outside the configured root")]
    OutsideRoot,
    #[error("backup manifest is unavailable")]
    Unavailable,
    #[error("backup manifest is invalid")]
    InvalidManifest,
    #[error("backup artifact is unavailable")]
    ArtifactUnavailable,
    #[error("backup digest does not match its manifest")]
    DigestMismatch,
    #[error("backup database failed integrity verification")]
    IntegrityMismatch,
    #[error("backup database exceeds configured size limit")]
    TooLarge,
    #[error("backup database verification failed")]
    Database,
    #[error("backup manifest verification failed")]
    Io,
}

/// SQLite backup adapter with bounded, fail-closed publication and retention.
#[derive(Clone)]
pub struct DatabaseBackupService {
    storage: SqliteStorage,
    policy: BackupPolicy,
}

#[derive(Clone, Copy)]
struct BackupPaths<'a> {
    backup_id: &'a str,
    database_file: &'a str,
    database_path: &'a Path,
    manifest_path: &'a Path,
    temporary_database_path: &'a Path,
    temporary_manifest_path: &'a Path,
}

impl DatabaseBackupService {
    pub fn new(storage: SqliteStorage, policy: BackupPolicy) -> Self {
        Self { storage, policy }
    }

    /// Create a consistent online snapshot and publish its manifest last.
    pub async fn create(&self, request: BackupRequest) -> Result<BackupArtifact, BackupError> {
        request.validate()?;
        let source_path = self
            .storage
            .config()
            .database_path
            .as_deref()
            .ok_or(BackupError::InMemorySource)?;
        if !tokio::fs::metadata(source_path)
            .await
            .map_err(|_| BackupError::SourceUnavailable)?
            .is_file()
        {
            return Err(BackupError::SourceUnavailable);
        }

        let root = self.canonical_root().await?;
        let backup_id = format!("backup-{}", Uuid::new_v4().simple());
        let database_file = format!("{backup_id}.db");
        let manifest_file = format!("{backup_id}.manifest.json");
        let database_path = root.join(&database_file);
        let manifest_path = root.join(&manifest_file);
        let temporary_database_path = root.join(format!("{database_file}.tmp"));
        let temporary_manifest_path = root.join(format!("{manifest_file}.tmp"));

        let paths = BackupPaths {
            backup_id: &backup_id,
            database_file: &database_file,
            database_path: &database_path,
            manifest_path: &manifest_path,
            temporary_database_path: &temporary_database_path,
            temporary_manifest_path: &temporary_manifest_path,
        };
        let result = self.create_files(&request, paths).await;

        if result.is_err() {
            let _ = remove_if_exists(paths.temporary_database_path).await;
            let _ = remove_if_exists(paths.temporary_manifest_path).await;
            let _ = remove_if_exists(paths.database_path).await;
            let _ = remove_if_exists(paths.manifest_path).await;
        }
        result
    }

    async fn create_files(
        &self,
        request: &BackupRequest,
        paths: BackupPaths<'_>,
    ) -> Result<BackupArtifact, BackupError> {
        if tokio::fs::try_exists(paths.database_path)
            .await
            .map_err(BackupError::Io)?
            || tokio::fs::try_exists(paths.manifest_path)
                .await
                .map_err(BackupError::Io)?
        {
            return Err(BackupError::Destination);
        }
        let temporary_database = path_string(paths.temporary_database_path)?;
        let vacuum_sql = format!("VACUUM INTO '{}'", sql_quote(&temporary_database));
        let source_config = self.storage.config().clone();
        let snapshot_storage = SqliteStorage::connect(SqliteStorageConfig {
            database_path: Some(
                source_config
                    .database_path
                    .clone()
                    .ok_or(BackupError::InMemorySource)?,
            ),
            max_connections: 1,
            busy_timeout: source_config.busy_timeout,
            create_if_missing: false,
            wal_mode: source_config.wal_mode,
            foreign_keys: source_config.foreign_keys,
        })
        .await
        .map_err(BackupError::Storage)?;
        let vacuum_result = sqlx::query(&vacuum_sql)
            .execute(snapshot_storage.pool())
            .await;
        snapshot_storage.close().await;
        vacuum_result.map_err(BackupError::Database)?;
        let database_file_handle = tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(paths.temporary_database_path)
            .await
            .map_err(|error| io_context("open snapshot", error))?;
        database_file_handle
            .sync_all()
            .await
            .map_err(|error| io_context("sync snapshot", error))?;
        drop(database_file_handle);

        let (database_size_bytes, database_sha256) =
            hash_file(paths.temporary_database_path, self.policy.max_backup_bytes)
                .await
                .map_err(|error| match error {
                    FileHashError::TooLarge => BackupError::TooLarge,
                    FileHashError::Io(error) => BackupError::Io(error),
                })?;

        let schema_version = schema_version(self.storage.pool())
            .await
            .map_err(BackupError::Database)?;
        let manifest = BackupManifest {
            format_version: BACKUP_FORMAT_VERSION,
            backup_id: paths.backup_id.to_owned(),
            profile_id: request.profile_id.clone(),
            schema_version,
            app_version: request.app_version.clone(),
            source_revision: request.source_revision.clone(),
            source_tree: request.source_tree.clone(),
            policy_revision: request.policy_revision.clone(),
            protection: request.protection.clone(),
            created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            database_file: paths.database_file.to_owned(),
            database_size_bytes,
            database_sha256: database_sha256.clone(),
        };
        let manifest_bytes = serde_json::to_vec(&manifest).map_err(BackupError::Serialization)?;
        if manifest_bytes.len() as u64 > MAX_MANIFEST_BYTES {
            return Err(BackupError::TooLarge);
        }

        let mut manifest_file = tokio::fs::File::create(paths.temporary_manifest_path)
            .await
            .map_err(|error| io_context("create manifest", error))?;
        manifest_file
            .write_all(&manifest_bytes)
            .await
            .map_err(|error| io_context("write manifest", error))?;
        manifest_file
            .sync_all()
            .await
            .map_err(|error| io_context("sync manifest", error))?;
        drop(manifest_file);

        tokio::fs::rename(paths.temporary_database_path, paths.database_path)
            .await
            .map_err(|error| io_context("publish database", error))?;
        tokio::fs::rename(paths.temporary_manifest_path, paths.manifest_path)
            .await
            .map_err(|error| io_context("publish manifest", error))?;

        Ok(BackupArtifact {
            manifest,
            database_path: paths.database_path.to_path_buf(),
            manifest_path: paths.manifest_path.to_path_buf(),
            database_size_bytes,
            database_sha256,
            manifest_sha256: digest_bytes(&manifest_bytes),
        })
    }

    /// Verify manifest identity, digest and SQLite structural integrity.
    pub async fn verify(
        &self,
        manifest_path: impl AsRef<Path>,
    ) -> Result<BackupVerification, VerificationError> {
        let root = self
            .canonical_root()
            .await
            .map_err(|_| VerificationError::Io)?;
        let requested_path =
            absolute_path(manifest_path.as_ref()).ok_or(VerificationError::OutsideRoot)?;
        if !requested_path.starts_with(&root) {
            return Err(VerificationError::OutsideRoot);
        }
        let manifest_path = tokio::fs::canonicalize(&requested_path)
            .await
            .map_err(|_| VerificationError::Unavailable)?;
        if !manifest_path.starts_with(&root)
            || !manifest_path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".manifest.json"))
        {
            return Err(VerificationError::OutsideRoot);
        }

        let metadata = tokio::fs::metadata(&manifest_path)
            .await
            .map_err(|_| VerificationError::Unavailable)?;
        if metadata.len() > MAX_MANIFEST_BYTES {
            return Err(VerificationError::InvalidManifest);
        }
        let manifest_bytes = tokio::fs::read(&manifest_path)
            .await
            .map_err(|_| VerificationError::Unavailable)?;
        let manifest: BackupManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|_| VerificationError::InvalidManifest)?;
        manifest.validate()?;

        let expected_manifest_name = format!("{}.manifest.json", manifest.backup_id);
        if manifest_path.file_name().and_then(|name| name.to_str())
            != Some(expected_manifest_name.as_str())
        {
            return Err(VerificationError::InvalidManifest);
        }
        if manifest.database_file != format!("{}.db", manifest.backup_id) {
            return Err(VerificationError::InvalidManifest);
        }
        let database_path = root.join(&manifest.database_file);
        let canonical_database_path = tokio::fs::canonicalize(&database_path)
            .await
            .map_err(|_| VerificationError::ArtifactUnavailable)?;
        if !canonical_database_path.starts_with(&root) {
            return Err(VerificationError::OutsideRoot);
        }
        let (size, digest) = hash_file(&canonical_database_path, self.policy.max_backup_bytes)
            .await
            .map_err(|error| match error {
                FileHashError::TooLarge => VerificationError::TooLarge,
                FileHashError::Io(_) => VerificationError::ArtifactUnavailable,
            })?;
        if size != manifest.database_size_bytes || digest != manifest.database_sha256 {
            return Err(VerificationError::DigestMismatch);
        }

        let storage = SqliteStorage::connect(SqliteStorageConfig {
            database_path: Some(canonical_database_path.clone()),
            max_connections: 1,
            busy_timeout: std::time::Duration::from_secs(5),
            create_if_missing: false,
            wal_mode: false,
            foreign_keys: true,
        })
        .await
        .map_err(|_| VerificationError::Database)?;
        let integrity = sqlx::query_scalar::<Sqlite, String>("PRAGMA integrity_check")
            .fetch_one(storage.pool())
            .await
            .map_err(|_| VerificationError::Database)?;
        storage.close().await;
        if integrity != "ok" {
            return Err(VerificationError::IntegrityMismatch);
        }

        Ok(BackupVerification {
            manifest,
            database_path: canonical_database_path,
            manifest_sha256: digest_bytes(&manifest_bytes),
        })
    }

    /// Retain the newest verified artifacts and delete only older generated pairs.
    pub async fn enforce_retention(&self) -> Result<RetentionReport, BackupError> {
        let root = self.canonical_root().await?;
        let mut entries = tokio::fs::read_dir(&root)
            .await
            .map_err(BackupError::Retention)?;
        let mut manifest_paths = Vec::new();
        while let Some(entry) = entries.next_entry().await.map_err(BackupError::Retention)? {
            if manifest_paths.len() == MAX_SCAN_ENTRIES {
                return Err(BackupError::Retention(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "backup directory exceeds bounded scan",
                )));
            }
            let path = entry.path();
            if entry
                .file_type()
                .await
                .map_err(BackupError::Retention)?
                .is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(".manifest.json"))
            {
                manifest_paths.push(path);
            }
        }

        let mut report = RetentionReport::default();
        let mut verified = Vec::new();
        for path in manifest_paths {
            match self.verify(&path).await {
                Ok(result) => verified.push((result.manifest, path)),
                Err(_) => report.skipped += 1,
            }
        }
        verified.sort_by(|(left, _), (right, _)| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.backup_id.cmp(&left.backup_id))
        });

        let mut retained_bytes = 0u64;
        for (manifest, manifest_path) in verified {
            if report.retained.len() < self.policy.max_backups
                && retained_bytes.saturating_add(manifest.database_size_bytes)
                    <= self.policy.max_total_bytes
            {
                retained_bytes = retained_bytes.saturating_add(manifest.database_size_bytes);
                report.retained.push(manifest.backup_id);
                continue;
            }
            let database_path = root.join(&manifest.database_file);
            tokio::fs::remove_file(&manifest_path)
                .await
                .map_err(BackupError::Retention)?;
            remove_if_exists(&database_path)
                .await
                .map_err(BackupError::Retention)?;
            report.deleted.push(manifest.backup_id);
        }
        Ok(report)
    }

    async fn canonical_root(&self) -> Result<PathBuf, BackupError> {
        if let Ok(metadata) = tokio::fs::symlink_metadata(&self.policy.destination_root).await {
            if metadata.file_type().is_symlink() {
                return Err(BackupError::Destination);
            }
        }
        tokio::fs::create_dir_all(&self.policy.destination_root)
            .await
            .map_err(BackupError::Io)?;
        if tokio::fs::symlink_metadata(&self.policy.destination_root)
            .await
            .map_err(BackupError::Io)?
            .file_type()
            .is_symlink()
        {
            return Err(BackupError::Destination);
        }
        tokio::fs::canonicalize(&self.policy.destination_root)
            .await
            .map_err(BackupError::Io)
    }
}

async fn schema_version(pool: &sqlx::Pool<Sqlite>) -> Result<i64, sqlx::Error> {
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

enum FileHashError {
    TooLarge,
    Io(std::io::Error),
}

async fn hash_file(path: &Path, limit: u64) -> Result<(u64, String), FileHashError> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(FileHashError::Io)?;
    let mut hasher = Sha256::new();
    let mut bytes_read = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).await.map_err(FileHashError::Io)?;
        if read == 0 {
            break;
        }
        bytes_read = bytes_read.saturating_add(read as u64);
        if bytes_read > limit {
            return Err(FileHashError::TooLarge);
        }
        hasher.update(&buffer[..read]);
    }
    Ok((bytes_read, hex_digest(hasher.finalize().as_slice())))
}

fn digest_bytes(bytes: &[u8]) -> String {
    hex_digest(Sha256::digest(bytes).as_slice())
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn valid_text(value: &str, max_bytes: usize) -> bool {
    !value.is_empty() && value.len() <= max_bytes && !value.chars().any(char::is_control)
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

fn is_safe_path(path: &Path) -> bool {
    !path.to_string_lossy().chars().any(char::is_control)
        && path
            .components()
            .all(|component| !matches!(component, Component::ParentDir))
}

fn absolute_path(path: &Path) -> Option<PathBuf> {
    if path.is_absolute() {
        Some(path.to_path_buf())
    } else {
        std::env::current_dir()
            .ok()
            .map(|current| current.join(path))
    }
}

fn path_string(path: &Path) -> Result<String, BackupError> {
    path.to_str()
        .map(ToOwned::to_owned)
        .ok_or(BackupError::Destination)
}

fn io_context(context: &str, error: std::io::Error) -> BackupError {
    BackupError::Io(std::io::Error::new(
        error.kind(),
        format!("{context}: {error}"),
    ))
}

fn sql_quote(value: &str) -> String {
    value.replace('\'', "''")
}

async fn remove_if_exists(path: &Path) -> Result<(), std::io::Error> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
