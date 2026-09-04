//! Fail-closed preflight and durable state for SQLite schema upgrades.
//!
//! SQLx remains the authority that executes migrations. This module adds the
//! boundary around it: the embedded source is exposed as a deterministic
//! manifest, applied history is checked before execution, risky upgrades need
//! a verified last-known-good backup, and only forward recovery is allowed.

use crate::backup::BackupVerification;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Row, Sqlite};
use std::borrow::Cow;
use thiserror::Error;

pub const MIGRATION_MANIFEST_FORMAT_VERSION: u16 = 1;
const MAX_OPERATION_ID_BYTES: usize = 136;
const MAX_PROFILE_ID_BYTES: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationManifestEntry {
    pub version: i64,
    pub description: String,
    pub checksum: String,
    pub transactional: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationManifest {
    pub format_version: u16,
    pub migrations: Vec<MigrationManifestEntry>,
}

impl MigrationManifest {
    pub fn latest_version(&self) -> i64 {
        self.migrations
            .last()
            .map_or(0, |migration| migration.version)
    }

    /// Digest of the versioned metadata, independent of JSON formatting.
    pub fn manifest_digest(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.format_version.to_string().as_bytes());
        hasher.update(b"\n");
        for migration in &self.migrations {
            hasher.update(migration.version.to_string().as_bytes());
            hasher.update(b"\0");
            hasher.update(migration.description.as_bytes());
            hasher.update(b"\0");
            hasher.update(migration.checksum.as_bytes());
            hasher.update(b"\0");
            hasher.update(if migration.transactional { b"1" } else { b"0" });
            hasher.update(b"\n");
        }
        hex_encode(&hasher.finalize())
    }
}

/// Build the manifest from the exact SQLx source embedded in this binary.
pub fn embedded_migration_manifest() -> MigrationManifest {
    let migrator = sqlx::migrate!("../../migrations");
    MigrationManifest {
        format_version: MIGRATION_MANIFEST_FORMAT_VERSION,
        migrations: migrator
            .iter()
            .map(|migration| MigrationManifestEntry {
                version: migration.version,
                description: migration.description.to_string(),
                checksum: hex_encode(&migration.checksum),
                transactional: !migration.no_tx,
            })
            .collect(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationAction {
    CleanInstall,
    UpToDate,
    Upgrade { from_version: i64, to_version: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationPreflight {
    pub current_version: i64,
    pub target_version: i64,
    pub manifest_digest: String,
    pub action: MigrationAction,
}

#[derive(Debug, Clone)]
pub struct MigrationRequest {
    pub operation_id: String,
    pub profile_id: String,
    pub target_version: i64,
    pub verified_backup: Option<BackupVerification>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationRunStatus {
    Applied,
    AlreadyApplied,
    Noop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationRunResult {
    pub status: MigrationRunStatus,
    pub action: MigrationAction,
    pub current_version: i64,
    pub target_version: i64,
    pub manifest_digest: String,
}

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("invalid migration request")]
    InvalidRequest,
    #[error("migration manifest is empty or unordered")]
    InvalidManifest,
    #[error("requested schema version is not supported: {target_version}")]
    UnsupportedTarget { target_version: i64 },
    #[error("database contains an unknown migration version: {version}")]
    UnknownAppliedMigration { version: i64 },
    #[error("database migration history skips version: {version}")]
    MissingMigration { version: i64 },
    #[error("database migration checksum drift at version: {version}")]
    ChecksumMismatch { version: i64 },
    #[error("database has a dirty migration: {version}")]
    DirtyMigration { version: i64 },
    #[error("database has schema tables without SQLx migration history")]
    UnsupportedSchema,
    #[error("schema downgrade is not supported: {current_version} to {target_version}")]
    DowngradeBlocked {
        current_version: i64,
        target_version: i64,
    },
    #[error("migration upgrade requires a verified backup of schema {current_version}")]
    BackupRequired { current_version: i64 },
    #[error(
        "verified backup schema does not match the migration starting point: {backup_version} != {current_version}"
    )]
    BackupSchemaMismatch {
        backup_version: i64,
        current_version: i64,
    },
    #[error("verified backup profile does not match the migration profile")]
    BackupProfileMismatch,
    #[error("migration history contains a non-transactional step")]
    NonTransactionalMigration { version: i64 },
    #[error("migration operation conflicts with an in-flight operation")]
    StateConflict { operation_id: String },
    #[error("migration execution failed")]
    ExecutionFailed { failure_class: &'static str },
    #[error("migration database operation failed")]
    Database(String),
}

pub async fn migration_preflight(
    pool: &Pool<Sqlite>,
    request: &MigrationRequest,
) -> Result<MigrationPreflight, MigrationError> {
    validate_request(request)?;
    let manifest = embedded_migration_manifest();
    validate_manifest(&manifest)?;

    let has_history = scalar_i64(
        pool,
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations')",
    )
    .await?
        != 0;

    let applied = if has_history {
        read_applied(pool).await?
    } else {
        let has_schema_tables = scalar_i64(
            pool,
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' AND name NOT IN ('_hank_migration_runs', '_sqlx_migrations'))",
        )
        .await?
            != 0;
        if has_schema_tables {
            return Err(MigrationError::UnsupportedSchema);
        }
        Vec::new()
    };

    validate_applied(&applied, &manifest)?;
    let current_version = applied.last().map_or(0, |migration| migration.0);
    if current_version > request.target_version {
        return Err(MigrationError::DowngradeBlocked {
            current_version,
            target_version: request.target_version,
        });
    }
    if request.target_version > manifest.latest_version()
        || (request.target_version != manifest.latest_version()
            && current_version < request.target_version)
    {
        return Err(MigrationError::UnsupportedTarget {
            target_version: request.target_version,
        });
    }

    let action = if current_version == 0 {
        MigrationAction::CleanInstall
    } else if current_version == request.target_version {
        MigrationAction::UpToDate
    } else {
        for migration in manifest
            .migrations
            .iter()
            .filter(|migration| migration.version > current_version)
            .filter(|migration| migration.version <= request.target_version)
        {
            if !migration.transactional {
                return Err(MigrationError::NonTransactionalMigration {
                    version: migration.version,
                });
            }
        }
        let backup = request
            .verified_backup
            .as_ref()
            .ok_or(MigrationError::BackupRequired { current_version })?;
        if backup.manifest.schema_version != current_version {
            return Err(MigrationError::BackupSchemaMismatch {
                backup_version: backup.manifest.schema_version,
                current_version,
            });
        }
        if backup.manifest.profile_id != request.profile_id {
            return Err(MigrationError::BackupProfileMismatch);
        }
        MigrationAction::Upgrade {
            from_version: current_version,
            to_version: request.target_version,
        }
    };

    Ok(MigrationPreflight {
        current_version,
        target_version: request.target_version,
        manifest_digest: manifest.manifest_digest(),
        action,
    })
}

/// Execute the existing SQLx runner only after the hardened gate succeeds.
pub async fn run_migrations_hardened(
    pool: &Pool<Sqlite>,
    request: MigrationRequest,
) -> Result<MigrationRunResult, MigrationError> {
    let preflight = migration_preflight(pool, &request).await?;
    ensure_state_table(pool).await?;
    let existing = read_state(pool, &request.operation_id).await?;
    if let Some((profile_id, from_version, target_version, digest, status)) = existing {
        if profile_id != request.profile_id
            || from_version != preflight.current_version
                && !matches!(preflight.action, MigrationAction::UpToDate)
        {
            return Err(MigrationError::StateConflict {
                operation_id: request.operation_id,
            });
        }
        if target_version != preflight.target_version || digest != preflight.manifest_digest {
            return Err(MigrationError::StateConflict {
                operation_id: request.operation_id,
            });
        }
        match status.as_str() {
            "applied" => {
                if preflight.current_version != preflight.target_version {
                    return Err(MigrationError::StateConflict {
                        operation_id: request.operation_id,
                    });
                }
                return Ok(MigrationRunResult {
                    status: MigrationRunStatus::AlreadyApplied,
                    action: preflight.action,
                    current_version: preflight.current_version,
                    target_version: preflight.target_version,
                    manifest_digest: preflight.manifest_digest,
                });
            }
            "started" => {
                if preflight.current_version == preflight.target_version {
                    mark_applied(pool, &request.operation_id).await?;
                    return Ok(MigrationRunResult {
                        status: MigrationRunStatus::AlreadyApplied,
                        action: preflight.action,
                        current_version: preflight.current_version,
                        target_version: preflight.target_version,
                        manifest_digest: preflight.manifest_digest,
                    });
                }
                return Err(MigrationError::StateConflict {
                    operation_id: request.operation_id,
                });
            }
            "failed" => {
                mark_started(
                    pool,
                    &request.operation_id,
                    &request.profile_id,
                    preflight.current_version,
                    preflight.target_version,
                    &preflight.manifest_digest,
                )
                .await?;
            }
            _ => {
                return Err(MigrationError::StateConflict {
                    operation_id: request.operation_id,
                });
            }
        }
    } else {
        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO _hank_migration_runs (operation_id, profile_id, from_version, target_version, manifest_digest, status, failure_class, started_at, updated_at) VALUES (?, ?, ?, ?, ?, 'started', NULL, ?, ?)",
        )
        .bind(&request.operation_id)
        .bind(&request.profile_id)
        .bind(preflight.current_version)
        .bind(preflight.target_version)
        .bind(&preflight.manifest_digest)
        .bind(now())
        .bind(now())
        .execute(pool)
        .await
        .map_err(db_error)?;
        if inserted.rows_affected() == 0 {
            return Err(MigrationError::StateConflict {
                operation_id: request.operation_id,
            });
        }
    }

    let migrator = sqlx::migrate!("../../migrations");
    let bounded_migrator = sqlx::migrate::Migrator {
        migrations: Cow::Owned(
            migrator
                .iter()
                .filter(|migration| migration.version <= preflight.target_version)
                .cloned()
                .collect(),
        ),
        ignore_missing: false,
        locking: true,
        no_tx: false,
    };
    let result = bounded_migrator.run(pool).await;
    if result.is_err() {
        sqlx::query(
            "UPDATE _hank_migration_runs SET status = 'failed', failure_class = 'sqlx_execution', updated_at = ? WHERE operation_id = ?",
        )
        .bind(now())
        .bind(&request.operation_id)
        .execute(pool)
        .await
        .map_err(db_error)?;
        return Err(MigrationError::ExecutionFailed {
            failure_class: "sqlx_execution",
        });
    }

    mark_applied(pool, &request.operation_id).await?;

    Ok(MigrationRunResult {
        status: MigrationRunStatus::Applied,
        action: preflight.action,
        current_version: preflight.target_version,
        target_version: preflight.target_version,
        manifest_digest: preflight.manifest_digest,
    })
}

fn validate_request(request: &MigrationRequest) -> Result<(), MigrationError> {
    if request.operation_id.is_empty()
        || request.operation_id.len() > MAX_OPERATION_ID_BYTES
        || !request
            .operation_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        || request.target_version <= 0
        || request.profile_id.is_empty()
        || request.profile_id.len() > MAX_PROFILE_ID_BYTES
        || request.profile_id.chars().any(char::is_control)
    {
        return Err(MigrationError::InvalidRequest);
    }
    Ok(())
}

fn validate_manifest(manifest: &MigrationManifest) -> Result<(), MigrationError> {
    if manifest.format_version != MIGRATION_MANIFEST_FORMAT_VERSION
        || manifest.migrations.is_empty()
        || !manifest
            .migrations
            .windows(2)
            .all(|window| window[0].version < window[1].version)
        || manifest
            .migrations
            .iter()
            .any(|migration| migration.version <= 0 || migration.checksum.len() != 96)
    {
        return Err(MigrationError::InvalidManifest);
    }
    Ok(())
}

async fn ensure_state_table(pool: &Pool<Sqlite>) -> Result<(), MigrationError> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _hank_migration_runs (operation_id TEXT PRIMARY KEY NOT NULL, profile_id TEXT NOT NULL, from_version INTEGER NOT NULL, target_version INTEGER NOT NULL, manifest_digest TEXT NOT NULL, status TEXT NOT NULL CHECK(status IN ('started', 'applied', 'failed')), failure_class TEXT, started_at TEXT NOT NULL, updated_at TEXT NOT NULL)",
    )
    .execute(pool)
    .await
    .map_err(db_error)?;
    Ok(())
}

async fn read_applied(pool: &Pool<Sqlite>) -> Result<Vec<(i64, Vec<u8>)>, MigrationError> {
    let rows =
        sqlx::query("SELECT version, success, checksum FROM _sqlx_migrations ORDER BY version")
            .fetch_all(pool)
            .await
            .map_err(db_error)?;
    let mut applied = Vec::with_capacity(rows.len());
    for row in rows {
        let version = row.try_get::<i64, _>("version").map_err(db_error)?;
        let success = row.try_get::<bool, _>("success").map_err(db_error)?;
        if !success {
            return Err(MigrationError::DirtyMigration { version });
        }
        let checksum = row.try_get::<Vec<u8>, _>("checksum").map_err(db_error)?;
        applied.push((version, checksum));
    }
    Ok(applied)
}

fn validate_applied(
    applied: &[(i64, Vec<u8>)],
    manifest: &MigrationManifest,
) -> Result<(), MigrationError> {
    for (index, (version, checksum)) in applied.iter().enumerate() {
        let expected = manifest
            .migrations
            .iter()
            .find(|migration| migration.version == *version)
            .ok_or(MigrationError::UnknownAppliedMigration { version: *version })?;
        if manifest
            .migrations
            .get(index)
            .map(|migration| migration.version)
            != Some(*version)
        {
            let missing = manifest
                .migrations
                .get(index)
                .map_or(*version, |migration| migration.version);
            return Err(MigrationError::MissingMigration { version: missing });
        }
        if hex_encode(checksum) != expected.checksum {
            return Err(MigrationError::ChecksumMismatch { version: *version });
        }
    }
    Ok(())
}

async fn read_state(
    pool: &Pool<Sqlite>,
    operation_id: &str,
) -> Result<Option<(String, i64, i64, String, String)>, MigrationError> {
    sqlx::query(
        "SELECT profile_id, from_version, target_version, manifest_digest, status FROM _hank_migration_runs WHERE operation_id = ?",
    )
    .bind(operation_id)
    .fetch_optional(pool)
    .await
    .map_err(db_error)?
    .map(|row| {
        Ok((
            row.try_get("profile_id").map_err(db_error)?,
            row.try_get("from_version").map_err(db_error)?,
            row.try_get("target_version").map_err(db_error)?,
            row.try_get("manifest_digest").map_err(db_error)?,
            row.try_get("status").map_err(db_error)?,
        ))
    })
    .transpose()
}

async fn mark_started(
    pool: &Pool<Sqlite>,
    operation_id: &str,
    profile_id: &str,
    from_version: i64,
    target_version: i64,
    manifest_digest: &str,
) -> Result<(), MigrationError> {
    sqlx::query(
        "UPDATE _hank_migration_runs SET profile_id = ?, from_version = ?, target_version = ?, manifest_digest = ?, status = 'started', failure_class = NULL, updated_at = ? WHERE operation_id = ?",
    )
    .bind(profile_id)
    .bind(from_version)
    .bind(target_version)
    .bind(manifest_digest)
    .bind(now())
    .bind(operation_id)
    .execute(pool)
    .await
    .map_err(db_error)?;
    Ok(())
}

async fn mark_applied(pool: &Pool<Sqlite>, operation_id: &str) -> Result<(), MigrationError> {
    sqlx::query(
        "UPDATE _hank_migration_runs SET status = 'applied', failure_class = NULL, updated_at = ? WHERE operation_id = ?",
    )
    .bind(now())
    .bind(operation_id)
    .execute(pool)
    .await
    .map_err(db_error)?;
    Ok(())
}

async fn scalar_i64(pool: &Pool<Sqlite>, query: &str) -> Result<i64, MigrationError> {
    sqlx::query_scalar::<Sqlite, i64>(query)
        .fetch_one(pool)
        .await
        .map_err(db_error)
}

fn db_error(error: impl std::fmt::Display) -> MigrationError {
    MigrationError::Database(error.to_string())
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}
