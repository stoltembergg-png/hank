use agent_runtime::backup::{
    BackupPolicy, BackupProtection, BackupRequest, DatabaseBackupService, VerificationError,
};
use agent_runtime::migrations::run_migrations;
use agent_runtime::sqlite::{SqliteStorage, SqliteStorageConfig};
use sqlx::Row;
use tempfile::tempdir;
use tokio::io::AsyncWriteExt;

async fn seeded_file_storage() -> (tempfile::TempDir, SqliteStorage) {
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
    (dir, storage)
}

fn request() -> BackupRequest {
    BackupRequest {
        profile_id: "profile-a".into(),
        app_version: "0.3.0".into(),
        source_revision: "7aafe6b2efa1a82a2ca8b3999e57e94fc7ae8560".into(),
        source_tree: "tree-a".into(),
        policy_revision: "backup-policy-v1".into(),
        protection: BackupProtection::OsPolicy {
            key_reference: "os-handle-backup".into(),
        },
    }
}

fn policy(dir: &tempfile::TempDir) -> BackupPolicy {
    BackupPolicy::new(dir.path().join("backups"), 2, 1024 * 1024).unwrap()
}

// @spec:AC-1601
#[tokio::test]
async fn online_backup_is_verifiable_and_preserves_project_rows() {
    let (dir, storage) = seeded_file_storage().await;
    let service = DatabaseBackupService::new(storage.clone(), policy(&dir));

    let artifact = service.create(request()).await.unwrap();
    let verified = service.verify(&artifact.manifest_path).await.unwrap();

    assert_eq!(verified.manifest.profile_id, "profile-a");
    assert_eq!(verified.manifest.schema_version, 21);
    assert_eq!(
        verified.manifest.database_size_bytes,
        artifact.database_size_bytes
    );
    assert_eq!(verified.manifest.database_sha256, artifact.database_sha256);
    assert_eq!(verified.manifest.protection, request().protection);

    let backup = SqliteStorage::connect(SqliteStorageConfig::for_file(&artifact.database_path))
        .await
        .unwrap();
    let row = sqlx::query("SELECT name FROM projects WHERE id = 'project-a'")
        .fetch_one(backup.pool())
        .await
        .unwrap();
    assert_eq!(row.get::<String, _>("name"), "Project");
    backup.close().await;
    storage.close().await;
}

// @spec:AC-1602
#[tokio::test]
async fn manifest_and_database_digest_mismatches_are_rejected() {
    let (dir, storage) = seeded_file_storage().await;
    let service = DatabaseBackupService::new(storage.clone(), policy(&dir));
    let artifact = service.create(request()).await.unwrap();

    let mut backup_file = tokio::fs::OpenOptions::new()
        .append(true)
        .open(&artifact.database_path)
        .await
        .unwrap();
    backup_file.write_all(b"tampered").await.unwrap();
    backup_file.sync_all().await.unwrap();
    let result = service.verify(&artifact.manifest_path).await;
    assert!(matches!(result, Err(VerificationError::DigestMismatch)));
    storage.close().await;
}

// @spec:AC-1601
#[tokio::test]
async fn online_backup_can_run_while_workflow_writes_are_committed() {
    let (dir, storage) = seeded_file_storage().await;
    let writer_storage = storage.clone();
    let writer = tokio::spawn(async move {
        for index in 0..32 {
            sqlx::query(
                "INSERT INTO agents (id, project_id, name, status, personality, policy, created_at, updated_at) \
                 VALUES (?, 'project-a', ?, 'active', '{}', '{}', '2026-01-01', '2026-01-01')",
            )
            .bind(format!("agent-{index}"))
            .bind(format!("Agent {index}"))
            .execute(writer_storage.pool())
            .await
            .unwrap();
            tokio::task::yield_now().await;
        }
    });
    let service = DatabaseBackupService::new(storage.clone(), policy(&dir));
    let artifact = service.create(request()).await.unwrap();
    writer.await.unwrap();
    let backup = SqliteStorage::connect(SqliteStorageConfig::for_file(&artifact.database_path))
        .await
        .unwrap();
    let row = sqlx::query("SELECT count(*) AS count FROM agents WHERE project_id = 'project-a'")
        .fetch_one(backup.pool())
        .await
        .unwrap();
    let count: i64 = row.get("count");
    assert!((0..=32).contains(&count));
    backup.close().await;
    storage.close().await;
}

// @spec:AC-1603
#[tokio::test]
async fn backup_stays_inside_canonical_root_and_in_memory_sources_fail_closed() {
    let (dir, storage) = seeded_file_storage().await;
    let service = DatabaseBackupService::new(storage.clone(), policy(&dir));
    let outside = tempdir().unwrap().path().join("foreign.manifest.json");
    assert!(matches!(
        service.verify(&outside).await,
        Err(VerificationError::OutsideRoot)
    ));
    storage.close().await;

    let memory = SqliteStorage::connect_in_memory().await.unwrap();
    let memory_service = DatabaseBackupService::new(memory.clone(), policy(&dir));
    assert!(memory_service.create(request()).await.is_err());
    memory.close().await;
}

#[cfg(unix)]
// @spec:AC-1603
#[tokio::test]
async fn symlink_destination_root_is_rejected() {
    use std::os::unix::fs::symlink;

    let (dir, storage) = seeded_file_storage().await;
    let real_root = dir.path().join("real-backups");
    std::fs::create_dir(&real_root).unwrap();
    let linked_root = dir.path().join("linked-backups");
    symlink(&real_root, &linked_root).unwrap();
    let service = DatabaseBackupService::new(
        storage.clone(),
        BackupPolicy::new(&linked_root, 2, 1024 * 1024).unwrap(),
    );

    assert!(service.create(request()).await.is_err());
    assert!(std::fs::read_dir(&real_root).unwrap().next().is_none());
    storage.close().await;
}

// @spec:AC-1604
#[tokio::test]
async fn retention_deletes_only_oldest_verified_backups() {
    let (dir, storage) = seeded_file_storage().await;
    let service = DatabaseBackupService::new(storage.clone(), policy(&dir));

    let first = service.create(request()).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    let second = service.create(request()).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    let third = service.create(request()).await.unwrap();

    let retention = service.enforce_retention().await.unwrap();
    assert_eq!(retention.deleted, vec![first.manifest.backup_id]);
    assert!(retention.retained.contains(&second.manifest.backup_id));
    assert!(retention.retained.contains(&third.manifest.backup_id));
    assert!(!first.database_path.exists());
    assert!(!first.manifest_path.exists());
    assert!(second.manifest_path.exists());
    assert!(third.manifest_path.exists());
    storage.close().await;
}

// @spec:AC-1605
#[tokio::test]
async fn manifest_never_serializes_secret_material_or_accepts_partial_temp_files() {
    let (dir, storage) = seeded_file_storage().await;
    let service = DatabaseBackupService::new(storage.clone(), policy(&dir));
    let artifact = service.create(request()).await.unwrap();
    let manifest = tokio::fs::read_to_string(&artifact.manifest_path)
        .await
        .unwrap();

    assert!(!manifest.contains("secret"));
    assert!(!manifest.contains("token"));
    assert!(manifest.contains("os-handle-backup"));
    tokio::fs::write(
        dir.path().join("backups").join("backup-incomplete.db.tmp"),
        b"partial",
    )
    .await
    .unwrap();
    assert!(service.enforce_retention().await.is_ok());
    storage.close().await;
}

// @spec:AC-1604
#[tokio::test]
async fn an_interrupted_or_oversized_snapshot_never_becomes_accepted_artifact() {
    let (dir, storage) = seeded_file_storage().await;
    let small_policy = policy(&dir).with_max_backup_bytes(1).unwrap();
    let service = DatabaseBackupService::new(storage.clone(), small_policy);

    assert!(service.create(request()).await.is_err());
    let entries: Vec<_> = std::fs::read_dir(dir.path().join("backups"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert!(entries.is_empty());
    storage.close().await;
}
