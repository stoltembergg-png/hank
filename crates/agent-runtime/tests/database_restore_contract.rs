use agent_runtime::backup::{BackupPolicy, BackupProtection, BackupRequest, DatabaseBackupService};
use agent_runtime::migrations::run_migrations;
use agent_runtime::restore::{
    restore_lock_path, restore_request_digest, DatabaseRestoreService, RestoreAuthorization,
    RestoreError, RestoreOutcome, RestorePolicy, RestoreRequest,
};
use agent_runtime::sqlite::{SqliteStorage, SqliteStorageConfig};
use sqlx::Row;
use tempfile::tempdir;

async fn seeded_source() -> (tempfile::TempDir, SqliteStorage, DatabaseBackupService) {
    let dir = tempdir().unwrap();
    let database_path = dir.path().join("profile.db");
    let storage = SqliteStorage::connect(SqliteStorageConfig::for_file(&database_path))
        .await
        .unwrap();
    run_migrations(storage.pool()).await.unwrap();
    sqlx::query(
        "INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) \
         VALUES ('project-a', 'Project', 'active', 'owner-a', '2026-01-01', '2026-01-01', '{}')",
    )
    .execute(storage.pool())
    .await
    .unwrap();
    let backup = DatabaseBackupService::new(
        storage.clone(),
        BackupPolicy::new(dir.path().join("backups"), 4, 1024 * 1024).unwrap(),
    );
    (dir, storage, backup)
}

fn backup_request() -> BackupRequest {
    BackupRequest {
        profile_id: "profile-a".into(),
        app_version: "0.3.0".into(),
        source_revision: "9a58e88b380069e46e801087a1d10ba8a3a28dfb".into(),
        source_tree: "tree-backup".into(),
        policy_revision: "backup-policy-v1".into(),
        protection: BackupProtection::OsPolicy {
            key_reference: "os-handle-backup".into(),
        },
    }
}

fn restore_request(
    artifact: &agent_runtime::backup::BackupArtifact,
    target: &std::path::Path,
    target_profile_id: &str,
    target_schema_version: i64,
    restore_id: &str,
    dry_run: bool,
) -> RestoreRequest {
    let mut request = RestoreRequest {
        restore_id: restore_id.into(),
        source_manifest_path: artifact.manifest_path.clone(),
        source_backup_id: artifact.manifest.backup_id.clone(),
        target_profile_id: target_profile_id.into(),
        target_database_path: target.into(),
        target_schema_version,
        authorization: RestoreAuthorization {
            actor_id: "operator-a".into(),
            confirmation_id: "confirmation-a".into(),
            request_digest: String::new(),
        },
        dry_run,
    };
    request.authorization.request_digest = restore_request_digest(&request);
    request
}

fn restore_service(
    backup: DatabaseBackupService,
    target_root: &std::path::Path,
) -> DatabaseRestoreService {
    DatabaseRestoreService::new(
        backup,
        RestorePolicy::new(target_root, 1024 * 1024, 21).unwrap(),
    )
}

// @spec:AC-1701
#[tokio::test]
async fn clean_restore_stages_migrates_and_promotes_an_isolated_profile() {
    let (dir, source_storage, backup) = seeded_source().await;
    let artifact = backup.create(backup_request()).await.unwrap();
    let target_root = dir.path().join("profiles");
    let target = target_root.join("profile-a.db");
    let service = restore_service(backup, &target_root);

    let result = service
        .restore(restore_request(
            &artifact,
            &target,
            "profile-a",
            21,
            "restore-clean",
            false,
        ))
        .await
        .unwrap();

    assert_eq!(result.outcome, RestoreOutcome::Applied);
    assert_eq!(result.schema_version, 21);
    let restored = SqliteStorage::connect(SqliteStorageConfig::for_file(&target))
        .await
        .unwrap();
    let row = sqlx::query("SELECT name FROM projects WHERE id = 'project-a'")
        .fetch_one(restored.pool())
        .await
        .unwrap();
    assert_eq!(row.get::<String, _>("name"), "Project");
    restored.close().await;
    source_storage.close().await;
}

// @spec:AC-1701
#[tokio::test]
async fn existing_target_is_replaced_and_receipt_makes_retry_idempotent() {
    let (dir, source_storage, backup) = seeded_source().await;
    let artifact = backup.create(backup_request()).await.unwrap();
    let target_root = dir.path().join("profiles");
    let target = target_root.join("profile-a.db");
    let old_target = SqliteStorage::connect(SqliteStorageConfig::for_file(&target))
        .await
        .unwrap();
    run_migrations(old_target.pool()).await.unwrap();
    sqlx::query(
        "INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) \
         VALUES ('project-a', 'Old project', 'active', 'owner-a', '2026-01-01', '2026-01-01', '{}')",
    )
    .execute(old_target.pool())
    .await
    .unwrap();
    let old_row = sqlx::query("SELECT name FROM projects WHERE id = 'project-a'")
        .fetch_one(old_target.pool())
        .await
        .unwrap();
    assert_eq!(old_row.get::<String, _>("name"), "Old project");
    old_target.close().await;

    let service = restore_service(backup, &target_root);
    let request = restore_request(
        &artifact,
        &target,
        "profile-a",
        21,
        "restore-replace",
        false,
    );
    let first = service.restore(request.clone()).await.unwrap();
    assert_eq!(first.outcome, RestoreOutcome::Applied);
    assert!(!target
        .with_file_name(".profile-a.db.restore-previous.db")
        .exists());

    let second = service.restore(request).await.unwrap();
    assert_eq!(second.outcome, RestoreOutcome::AlreadyApplied);
    assert_eq!(first.target_sha256, second.target_sha256);

    let restored = SqliteStorage::connect(SqliteStorageConfig::for_file(&target))
        .await
        .unwrap();
    let row = sqlx::query("SELECT name FROM projects WHERE id = 'project-a'")
        .fetch_one(restored.pool())
        .await
        .unwrap();
    assert_eq!(row.get::<String, _>("name"), "Project");
    restored.close().await;
    source_storage.close().await;
}

// @spec:AC-1701
#[tokio::test]
async fn older_backup_is_migrated_during_staging_before_promotion() {
    let (dir, source_storage, backup) = seeded_source().await;
    sqlx::query("DROP TABLE task_workspace_mappings")
        .execute(source_storage.pool())
        .await
        .unwrap();
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 21")
        .execute(source_storage.pool())
        .await
        .unwrap();
    let artifact = backup.create(backup_request()).await.unwrap();
    assert_eq!(artifact.manifest.schema_version, 20);
    let target_root = dir.path().join("profiles");
    let target = target_root.join("profile-a.db");
    let service = restore_service(backup, &target_root);

    let result = service
        .restore(restore_request(
            &artifact,
            &target,
            "profile-a",
            21,
            "restore-upgrade",
            false,
        ))
        .await
        .unwrap();

    assert_eq!(result.outcome, RestoreOutcome::Applied);
    assert!(result.requires_migration);
    assert_eq!(result.schema_version, 21);
    source_storage.close().await;
}

// @spec:AC-1702
#[tokio::test]
async fn dry_run_reports_incompatible_schema_without_writing_target() {
    let (dir, source_storage, backup) = seeded_source().await;
    let artifact = backup.create(backup_request()).await.unwrap();
    let target_root = dir.path().join("profiles");
    let target = target_root.join("profile-a.db");
    let service = restore_service(backup, &target_root);

    let result = service
        .restore(restore_request(
            &artifact,
            &target,
            "profile-a",
            20,
            "restore-dry-run",
            true,
        ))
        .await
        .unwrap();

    assert_eq!(result.outcome, RestoreOutcome::DryRun);
    assert!(!result.compatible);
    assert!(!target.exists());
    source_storage.close().await;
}

// @spec:AC-1702
#[tokio::test]
async fn incompatible_schema_is_rejected_before_staging() {
    let (dir, source_storage, backup) = seeded_source().await;
    let artifact = backup.create(backup_request()).await.unwrap();
    let target_root = dir.path().join("profiles");
    let target = target_root.join("profile-a.db");
    let service = restore_service(backup, &target_root);

    let result = service
        .restore(restore_request(
            &artifact,
            &target,
            "profile-a",
            20,
            "restore-incompatible",
            false,
        ))
        .await;

    assert!(matches!(
        result,
        Err(RestoreError::IncompatibleSchema { .. })
    ));
    assert!(!target.exists());
    source_storage.close().await;
}

// @spec:AC-1703
#[tokio::test]
async fn digest_mismatch_never_touches_the_explicit_target() {
    let (dir, source_storage, backup) = seeded_source().await;
    let artifact = backup.create(backup_request()).await.unwrap();
    tokio::fs::write(&artifact.database_path, b"tampered")
        .await
        .unwrap();
    let target_root = dir.path().join("profiles");
    let target = target_root.join("profile-a.db");
    let service = restore_service(backup, &target_root);

    let result = service
        .restore(restore_request(
            &artifact,
            &target,
            "profile-a",
            21,
            "restore-digest",
            false,
        ))
        .await;

    assert!(matches!(
        result,
        Err(RestoreError::SourceVerification(
            agent_runtime::backup::VerificationError::DigestMismatch
        ))
    ));
    assert!(!target.exists());
    source_storage.close().await;
}

// @spec:AC-1703
#[tokio::test]
async fn duplicate_restore_returns_the_durable_result_without_second_promotion() {
    let (dir, source_storage, backup) = seeded_source().await;
    let artifact = backup.create(backup_request()).await.unwrap();
    let target_root = dir.path().join("profiles");
    let target = target_root.join("profile-a.db");
    let service = restore_service(backup, &target_root);
    let request = restore_request(&artifact, &target, "profile-a", 21, "restore-retry", false);

    let first = service.restore(request.clone()).await.unwrap();
    let second = service.restore(request).await.unwrap();

    assert_eq!(first.outcome, RestoreOutcome::Applied);
    assert_eq!(second.outcome, RestoreOutcome::AlreadyApplied);
    assert_eq!(first.target_sha256, second.target_sha256);

    let mut different_actor =
        restore_request(&artifact, &target, "profile-a", 21, "restore-retry", false);
    different_actor.authorization.actor_id = "operator-b".into();
    different_actor.authorization.request_digest = restore_request_digest(&different_actor);
    assert!(matches!(
        service.restore(different_actor).await,
        Err(RestoreError::RestoreConflict)
    ));
    source_storage.close().await;
}

// @spec:AC-1704
#[tokio::test]
async fn locked_target_is_rejected_without_staging() {
    let (dir, source_storage, backup) = seeded_source().await;
    let artifact = backup.create(backup_request()).await.unwrap();
    let target_root = dir.path().join("profiles");
    tokio::fs::create_dir_all(&target_root).await.unwrap();
    let target = target_root.join("profile-a.db");
    let lock = restore_lock_path(&target).unwrap();
    tokio::fs::write(&lock, b"active-restore").await.unwrap();
    let service = restore_service(backup, &target_root);

    let result = service
        .restore(restore_request(
            &artifact,
            &target,
            "profile-a",
            21,
            "restore-locked",
            false,
        ))
        .await;

    assert!(matches!(result, Err(RestoreError::TargetLocked)));
    assert!(!target.exists());
    tokio::fs::remove_file(lock).await.unwrap();
    source_storage.close().await;
}

// @spec:AC-1704
#[tokio::test]
async fn outside_target_and_profile_mismatch_fail_closed() {
    let (dir, source_storage, backup) = seeded_source().await;
    let artifact = backup.create(backup_request()).await.unwrap();
    let target_root = dir.path().join("profiles");
    let outside = dir.path().join("outside.db");
    let service = restore_service(backup.clone(), &target_root);

    let outside_result = service
        .restore(restore_request(
            &artifact,
            &outside,
            "profile-a",
            21,
            "restore-outside",
            false,
        ))
        .await;
    assert!(matches!(
        outside_result,
        Err(RestoreError::TargetOutsideRoot)
    ));

    let mismatch = target_root.join("other.db");
    let mismatch_result = service
        .restore(restore_request(
            &artifact,
            &mismatch,
            "profile-b",
            21,
            "restore-profile",
            false,
        ))
        .await;
    assert!(matches!(
        mismatch_result,
        Err(RestoreError::ProfileMismatch)
    ));
    source_storage.close().await;
}

#[cfg(unix)]
// @spec:AC-1704
#[tokio::test]
async fn symlink_target_is_rejected_without_following_the_link() {
    use std::os::unix::fs::symlink;

    let (dir, source_storage, backup) = seeded_source().await;
    let artifact = backup.create(backup_request()).await.unwrap();
    let target_root = dir.path().join("profiles");
    tokio::fs::create_dir_all(&target_root).await.unwrap();
    let foreign = dir.path().join("foreign.db");
    let target = target_root.join("profile-a.db");
    symlink(&foreign, &target).unwrap();
    let service = restore_service(backup, &target_root);

    let result = service
        .restore(restore_request(
            &artifact,
            &target,
            "profile-a",
            21,
            "restore-symlink",
            false,
        ))
        .await;

    assert!(matches!(result, Err(RestoreError::TargetSymlink)));
    assert!(!foreign.exists());
    source_storage.close().await;
}

// @spec:AC-1705
#[tokio::test]
async fn oversized_restore_cleans_staging_and_leaves_no_target() {
    let (dir, source_storage, backup) = seeded_source().await;
    let artifact = backup.create(backup_request()).await.unwrap();
    let target_root = dir.path().join("profiles");
    let target = target_root.join("profile-a.db");
    let service =
        DatabaseRestoreService::new(backup, RestorePolicy::new(&target_root, 1, 21).unwrap());

    let result = service
        .restore(restore_request(
            &artifact,
            &target,
            "profile-a",
            21,
            "restore-too-large",
            false,
        ))
        .await;

    assert!(matches!(result, Err(RestoreError::TooLarge)));
    assert!(!target.exists());
    let entries: Vec<_> = if target_root.exists() {
        std::fs::read_dir(&target_root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect()
    } else {
        Vec::new()
    };
    assert!(entries.is_empty());
    source_storage.close().await;
}
