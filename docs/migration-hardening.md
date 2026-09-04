# Migration hardening (PR-255)

`agent_runtime::migration_hardening` is the gate around the embedded SQLx migration
runner. It does not replace the migrations or provide a destructive downgrade path.

## Gate

1. The runtime builds an ordered manifest containing each migration version,
   description, SQLx checksum and transaction flag. Its SHA-256 digest is stable for
   the embedded source.
2. Preflight reads `_sqlx_migrations` and rejects dirty history, unknown or skipped
   versions, checksum drift, schema tables without SQLx history, unsupported targets
   and downgrades.
3. A clean install may run without a backup. An upgrade must carry a backup already
   verified by `DatabaseBackupService`, whose manifest schema version equals the
   observed starting version.
4. Execution records only bounded operation metadata in `_hank_migration_runs`:
   `started`, `applied` or `failed`. The operation ID is the idempotency key; an
   in-flight duplicate is rejected and an applied retry returns `AlreadyApplied`.
5. SQLx applies each existing migration transactionally. A failed attempt remains a
   failed forward-only operation: repair with a validated forward-fix or restore the
   last-known-good backup. The gate never calls a down migration.

The desktop bootstrap performs the preflight first. For an existing profile that needs
an upgrade, it creates and verifies a bounded snapshot under the application data
directory before invoking the gate. If snapshot publication or verification fails,
startup remains blocked and no migration is attempted.

Example:

```rust,no_run
use agent_runtime::{
    embedded_migration_manifest, run_migrations_hardened, BackupVerification, MigrationError,
    MigrationRequest, SqliteStorage,
};

async fn apply(
    storage: &SqliteStorage,
    verified_backup: BackupVerification,
) -> Result<(), MigrationError> {
let request = MigrationRequest {
    operation_id: "profile-upgrade-2026-09-04".into(),
    profile_id: "profile-a".into(),
    target_version: embedded_migration_manifest().latest_version(),
    verified_backup: Some(verified_backup),
};
    run_migrations_hardened(storage.pool(), request).await?;
    Ok(())
}
```

The API stores no SQL, prompts, credentials or secret values. Crash/power-loss,
disk-full simulation, OS-specific keychain behavior and a production migration run
remain outside this contract and are not claimed by its offline tests.
