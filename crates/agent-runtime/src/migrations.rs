//! Executador de migrações SQL e controle de versão de schema.
//!
//! Conforme PR-026 e regras de isolamento e idempotência.

use crate::sqlite::SqliteError;
use sqlx::{Pool, Sqlite};

/// Executa todas as migrações SQL pendentes no pool SQLite fornecido.
pub async fn run_migrations(pool: &Pool<Sqlite>) -> Result<(), SqliteError> {
    sqlx::migrate!("../../migrations")
        .run(pool)
        .await
        .map_err(|e| SqliteError::QueryError(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::SqliteStorage;
    use sqlx::Row;

    #[tokio::test]
    async fn run_migrations_on_clean_db_creates_expected_schema() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();

        // Verifica que as tabelas foram criadas
        let row = sqlx::query(
            "SELECT count(*) as count FROM sqlite_master WHERE type='table' AND name='projects';",
        )
        .fetch_one(storage.pool())
        .await
        .unwrap();
        let count: i64 = row.get("count");
        assert_eq!(count, 1);

        let row = sqlx::query(
            "SELECT count(*) as count FROM sqlite_master WHERE type='table' AND name='agents';",
        )
        .fetch_one(storage.pool())
        .await
        .unwrap();
        let count: i64 = row.get("count");
        assert_eq!(count, 1);

        let row = sqlx::query(
            "SELECT count(*) as count FROM sqlite_master WHERE type='table' AND name='sessions';",
        )
        .fetch_one(storage.pool())
        .await
        .unwrap();
        let count: i64 = row.get("count");
        assert_eq!(count, 1);

        let row = sqlx::query(
            "SELECT count(*) as count FROM sqlite_master WHERE type='table' AND name='messages';",
        )
        .fetch_one(storage.pool())
        .await
        .unwrap();
        let count: i64 = row.get("count");
        assert_eq!(count, 1);

        storage.close().await;
    }

    #[tokio::test]
    async fn run_migrations_is_idempotent() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        // Primeira execução
        run_migrations(storage.pool()).await.unwrap();
        // Segunda execução não deve falhar
        run_migrations(storage.pool()).await.unwrap();

        storage.close().await;
    }

    #[tokio::test]
    async fn foreign_keys_cascade_deletes_when_project_is_deleted() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();

        // Insere um projeto
        sqlx::query(
            "INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) \
             VALUES ('proj-1', 'Project Test', 'active', 'owner-1', '2026-01-01', '2026-01-01', '{}');"
        )
        .execute(storage.pool())
        .await
        .unwrap();

        // Insere um agente associado
        sqlx::query(
            "INSERT INTO agents (id, project_id, name, status, personality, policy, created_at, updated_at) \
             VALUES ('agent-1', 'proj-1', 'Agent Test', 'active', '{}', '{}', '2026-01-01', '2026-01-01');"
        )
        .execute(storage.pool())
        .await
        .unwrap();

        // Deleta o projeto
        sqlx::query("DELETE FROM projects WHERE id = 'proj-1';")
            .execute(storage.pool())
            .await
            .unwrap();

        // Agente deve ter sido removido por CASCADE
        let row = sqlx::query("SELECT count(*) as count FROM agents WHERE id = 'agent-1';")
            .fetch_one(storage.pool())
            .await
            .unwrap();
        let count: i64 = row.get("count");
        assert_eq!(count, 0);

        storage.close().await;
    }
}
