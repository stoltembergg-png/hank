use agent_runtime::backup::{BackupPolicy, BackupProtection, BackupRequest, DatabaseBackupService};
use agent_runtime::migration_hardening::{
    embedded_migration_manifest, migration_preflight, run_migrations_hardened, MigrationAction,
    MigrationError, MigrationRequest, MigrationRunStatus,
};
use agent_runtime::migrations::run_migrations;
use agent_runtime::sqlite::{SqliteStorage, SqliteStorageConfig};
use tempfile::tempdir;

fn request(operation_id: &str, target_version: i64) -> MigrationRequest {
    MigrationRequest {
        operation_id: operation_id.into(),
        profile_id: "profile-a".into(),
        target_version,
        verified_backup: None,
    }
}

async fn upgrade_fixture() -> (
    tempfile::TempDir,
    SqliteStorage,
    agent_runtime::backup::BackupVerification,
) {
    let dir = tempdir().unwrap();
    let database_path = dir.path().join("profile.db");
    let storage = SqliteStorage::connect(SqliteStorageConfig::for_file(&database_path))
        .await
        .unwrap();
    run_migrations(storage.pool()).await.unwrap();
    sqlx::query("DROP TABLE task_workspace_mappings")
        .execute(storage.pool())
        .await
        .unwrap();
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 21")
        .execute(storage.pool())
        .await
        .unwrap();
    let backup = DatabaseBackupService::new(
        storage.clone(),
        BackupPolicy::new(dir.path().join("backups"), 4, 1024 * 1024).unwrap(),
    );
    let artifact = backup
        .create(BackupRequest {
            profile_id: "profile-a".into(),
            app_version: "0.3.0".into(),
            source_revision: "migration-test".into(),
            source_tree: "migration-tree".into(),
            policy_revision: "migration-policy-v1".into(),
            protection: BackupProtection::OsPolicy {
                key_reference: "os-handle-migration".into(),
            },
        })
        .await
        .unwrap();
    let verified = backup.verify(&artifact.manifest_path).await.unwrap();
    (dir, storage, verified)
}

// @spec:AC-1801
#[test]
fn manifest_is_ordered_and_digest_is_deterministic() {
    let first = embedded_migration_manifest();
    let second = embedded_migration_manifest();

    assert_eq!(first, second);
    assert!(first
        .migrations
        .windows(2)
        .all(|window| window[0].version < window[1].version));
    assert_eq!(first.migrations.len(), 21);
    assert!(first.manifest_digest().len() == 64);
    assert!(first
        .migrations
        .iter()
        .all(|migration| migration.checksum.len() == 96));
}

// @spec:AC-1802
#[tokio::test]
async fn clean_install_and_current_schema_are_allowed_without_backup() {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    let target = embedded_migration_manifest().latest_version();

    let clean = migration_preflight(storage.pool(), &request("clean-install", target))
        .await
        .unwrap();
    assert_eq!(clean.action, MigrationAction::CleanInstall);

    let applied = run_migrations_hardened(storage.pool(), request("clean-install", target))
        .await
        .unwrap();
    assert_eq!(applied.status, MigrationRunStatus::Applied);

    let current = migration_preflight(storage.pool(), &request("current", target))
        .await
        .unwrap();
    assert_eq!(current.action, MigrationAction::UpToDate);
}

// @spec:AC-1802
#[tokio::test]
async fn pending_upgrade_requires_backup_from_observed_schema() {
    let (_dir, storage, _verified) = upgrade_fixture().await;
    let target = embedded_migration_manifest().latest_version();

    let result =
        migration_preflight(storage.pool(), &request("upgrade-without-proof", target)).await;

    assert!(matches!(result, Err(MigrationError::BackupRequired { .. })));
    let table_exists: i64 = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'task_workspace_mappings')",
    )
    .fetch_one(storage.pool())
    .await
    .unwrap();
    assert_eq!(table_exists, 0);
}

// @spec:AC-1803
#[tokio::test]
async fn checksum_drift_and_downgrade_are_rejected_before_execution() {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let target = embedded_migration_manifest().latest_version();

    sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = 1")
        .bind(vec![0_u8; 48])
        .execute(storage.pool())
        .await
        .unwrap();
    let drift = migration_preflight(storage.pool(), &request("drift", target)).await;
    assert!(matches!(
        drift,
        Err(MigrationError::ChecksumMismatch { version: 1 })
    ));

    let downgrade_storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(downgrade_storage.pool()).await.unwrap();
    let downgrade =
        migration_preflight(downgrade_storage.pool(), &request("downgrade", target - 1)).await;
    assert!(matches!(
        downgrade,
        Err(MigrationError::DowngradeBlocked {
            current_version: 21,
            target_version: 20
        })
    ));
}

// @spec:AC-1803
#[tokio::test]
async fn domain_schema_without_sqlx_history_is_unsupported() {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    sqlx::query("CREATE TABLE projects (id TEXT PRIMARY KEY NOT NULL)")
        .execute(storage.pool())
        .await
        .unwrap();

    let result = migration_preflight(
        storage.pool(),
        &request(
            "unsupported-history",
            embedded_migration_manifest().latest_version(),
        ),
    )
    .await;
    assert!(matches!(result, Err(MigrationError::UnsupportedSchema)));
}

// @spec:AC-1803
#[tokio::test]
async fn unknown_missing_and_dirty_history_are_rejected() {
    let unknown = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(unknown.pool()).await.unwrap();
    sqlx::query(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) VALUES (999, 'unknown', TRUE, ?, 0)",
    )
    .bind(vec![0_u8; 48])
    .execute(unknown.pool())
    .await
    .unwrap();
    let unknown_result = migration_preflight(
        unknown.pool(),
        &request(
            "unknown-history",
            embedded_migration_manifest().latest_version(),
        ),
    )
    .await;
    assert!(matches!(
        unknown_result,
        Err(MigrationError::UnknownAppliedMigration { version: 999 })
    ));

    let missing = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(missing.pool()).await.unwrap();
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 20")
        .execute(missing.pool())
        .await
        .unwrap();
    let missing_result = migration_preflight(
        missing.pool(),
        &request(
            "missing-history",
            embedded_migration_manifest().latest_version(),
        ),
    )
    .await;
    assert!(matches!(
        missing_result,
        Err(MigrationError::MissingMigration { version: 20 })
    ));

    let dirty = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(dirty.pool()).await.unwrap();
    sqlx::query("UPDATE _sqlx_migrations SET success = FALSE WHERE version = 21")
        .execute(dirty.pool())
        .await
        .unwrap();
    let dirty_result = migration_preflight(
        dirty.pool(),
        &request(
            "dirty-history",
            embedded_migration_manifest().latest_version(),
        ),
    )
    .await;
    assert!(matches!(
        dirty_result,
        Err(MigrationError::DirtyMigration { version: 21 })
    ));
}

// @spec:AC-1804
// @spec:AC-1805
#[tokio::test]
async fn upgrade_records_applied_and_repeated_request_is_idempotent() {
    let (_dir, storage, verified) = upgrade_fixture().await;
    let target = embedded_migration_manifest().latest_version();
    let mut migration_request = request("upgrade-once", target);
    migration_request.verified_backup = Some(verified);

    let first = run_migrations_hardened(storage.pool(), migration_request.clone())
        .await
        .unwrap();
    assert_eq!(first.status, MigrationRunStatus::Applied);
    sqlx::query(
        "UPDATE _hank_migration_runs SET status = 'started' WHERE operation_id = 'upgrade-once'",
    )
    .execute(storage.pool())
    .await
    .unwrap();
    let second = run_migrations_hardened(storage.pool(), migration_request)
        .await
        .unwrap();
    assert_eq!(second.status, MigrationRunStatus::AlreadyApplied);

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM _hank_migration_runs WHERE operation_id = 'upgrade-once'",
    )
    .fetch_one(storage.pool())
    .await
    .unwrap();
    assert_eq!(count, 1);
}

// @spec:AC-1804
#[tokio::test]
async fn failed_upgrade_is_recorded_without_claiming_success() {
    let (_dir, storage, verified) = upgrade_fixture().await;
    sqlx::query("CREATE TABLE task_workspace_mappings (project_id TEXT NOT NULL)")
        .execute(storage.pool())
        .await
        .unwrap();
    let target = embedded_migration_manifest().latest_version();
    let mut migration_request = request("upgrade-failure", target);
    migration_request.verified_backup = Some(verified);

    let result = run_migrations_hardened(storage.pool(), migration_request).await;
    assert!(matches!(
        result,
        Err(MigrationError::ExecutionFailed { .. })
    ));

    let status: String = sqlx::query_scalar(
        "SELECT status FROM _hank_migration_runs WHERE operation_id = 'upgrade-failure'",
    )
    .fetch_one(storage.pool())
    .await
    .unwrap();
    assert_eq!(status, "failed");
}

// @spec:AC-1806
#[tokio::test]
async fn duplicate_operation_id_is_not_applied_twice() {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    let target = embedded_migration_manifest().latest_version();
    let first = request("same-operation", target);
    let second = first.clone();
    let (left, right) = tokio::join!(
        run_migrations_hardened(storage.pool(), first),
        run_migrations_hardened(storage.pool(), second)
    );
    assert!(
        (left.is_ok() && right.is_ok())
            || (left.is_ok() && matches!(right, Err(MigrationError::StateConflict { .. })))
            || (right.is_ok() && matches!(left, Err(MigrationError::StateConflict { .. }))),
        "left={left:?}, right={right:?}"
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM _hank_migration_runs WHERE operation_id = 'same-operation'",
    )
    .fetch_one(storage.pool())
    .await
    .unwrap();
    assert_eq!(count, 1);
}
