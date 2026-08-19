//! Gerenciador de conexão SQLite transacional e seguro para agent-runtime.
//!
//! Conforme PR-025 e regras de fronteira arquitetural (AI-001, AI-003, D-001).
//! Proíbe acesso direto do frontend e isola persistência de dados de metadados.

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Executor, Pool, Sqlite, Transaction};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;
use thiserror::Error;

/// Erros possíveis na camada de armazenamento SQLite.
#[derive(Debug, Error)]
pub enum SqliteError {
    #[error("caminho de banco inválido ou inseguro: {0}")]
    InvalidPath(String),

    #[error("falha ao conectar ao banco de dados: {0}")]
    ConnectionFailed(String),

    #[error("falha em transação de banco de dados: {0}")]
    TransactionError(String),

    #[error("falha na execução de query: {0}")]
    QueryError(String),

    #[error("erro de I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("erro interno sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
}

/// Configurações da conexão SQLite.
#[derive(Debug, Clone)]
pub struct SqliteStorageConfig {
    /// Caminho do arquivo de banco de dados (None para banco em memória).
    pub database_path: Option<PathBuf>,
    /// Máximo de conexões no pool.
    pub max_connections: u32,
    /// Timeout para busy handler.
    pub busy_timeout: Duration,
    /// Se true, cria o arquivo se não existir.
    pub create_if_missing: bool,
    /// Ativar modo WAL.
    pub wal_mode: bool,
    /// Ativar enforce de Foreign Keys.
    pub foreign_keys: bool,
}

impl Default for SqliteStorageConfig {
    fn default() -> Self {
        Self {
            database_path: None,
            max_connections: 5,
            busy_timeout: Duration::from_secs(5),
            create_if_missing: true,
            wal_mode: true,
            foreign_keys: true,
        }
    }
}

impl SqliteStorageConfig {
    /// Configuração para banco em memória isolado (dev/testes).
    pub fn in_memory() -> Self {
        Self {
            database_path: None,
            max_connections: 1,
            busy_timeout: Duration::from_secs(5),
            create_if_missing: true,
            wal_mode: false,
            foreign_keys: true,
        }
    }

    /// Configuração para arquivo no sistema de arquivos.
    pub fn for_file(path: impl Into<PathBuf>) -> Self {
        Self {
            database_path: Some(path.into()),
            ..Default::default()
        }
    }
}

/// Valida segurança do caminho do banco contra path traversal e caracteres perigosos.
pub fn validate_sqlite_path(path: &Path) -> Result<(), SqliteError> {
    let path_str = path.to_string_lossy();
    if path_str.contains("..") {
        return Err(SqliteError::InvalidPath(
            "path traversal (..) não permitido".into(),
        ));
    }
    if path_str.chars().any(char::is_control) {
        return Err(SqliteError::InvalidPath(
            "caracteres de controle não permitidos no caminho".into(),
        ));
    }
    Ok(())
}

/// Gerenciador de armazenamento SQLite baseado em connection pool sqlx.
#[derive(Clone)]
pub struct SqliteStorage {
    pool: Pool<Sqlite>,
    config: SqliteStorageConfig,
}

impl SqliteStorage {
    /// Inicializa a conexão SQLite a partir da configuração fornecida.
    pub async fn connect(config: SqliteStorageConfig) -> Result<Self, SqliteError> {
        let connect_options = if let Some(ref path) = config.database_path {
            validate_sqlite_path(path)?;

            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    std::fs::create_dir_all(parent)?;
                }
            }

            let path_str = path.to_string_lossy();
            let mut opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path_str))
                .map_err(|e| SqliteError::ConnectionFailed(e.to_string()))?
                .create_if_missing(config.create_if_missing)
                .busy_timeout(config.busy_timeout);

            if config.wal_mode {
                opts = opts.journal_mode(SqliteJournalMode::Wal);
            }
            if config.foreign_keys {
                opts = opts.foreign_keys(true);
            }
            opts = opts.synchronous(SqliteSynchronous::Normal);
            opts
        } else {
            // In-memory
            let mut opts = SqliteConnectOptions::from_str("sqlite::memory:")
                .map_err(|e| SqliteError::ConnectionFailed(e.to_string()))?
                .busy_timeout(config.busy_timeout);

            if config.foreign_keys {
                opts = opts.foreign_keys(true);
            }
            opts
        };

        let pool = SqlitePoolOptions::new()
            .max_connections(config.max_connections.max(1))
            .acquire_timeout(config.busy_timeout)
            .connect_with(connect_options)
            .await
            .map_err(|e| SqliteError::ConnectionFailed(e.to_string()))?;

        // Executa pragmas iniciais para garantir integridade
        if config.foreign_keys {
            pool.execute("PRAGMA foreign_keys = ON;")
                .await
                .map_err(|e| SqliteError::QueryError(e.to_string()))?;
        }

        Ok(Self { pool, config })
    }

    /// Inicializa um banco de dados em memória efêmero e isolado.
    pub async fn connect_in_memory() -> Result<Self, SqliteError> {
        Self::connect(SqliteStorageConfig::in_memory()).await
    }

    /// Retorna uma referência ao pool de conexões do sqlx.
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    /// Inicia uma transação no banco.
    pub async fn begin(&self) -> Result<Transaction<'static, Sqlite>, SqliteError> {
        self.pool
            .begin()
            .await
            .map_err(|e| SqliteError::TransactionError(e.to_string()))
    }

    /// Executa verificação de conectividade simples (health check).
    pub async fn ping(&self) -> Result<(), SqliteError> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| SqliteError::QueryError(e.to_string()))?;
        Ok(())
    }

    /// Encerra todas as conexões ativas do pool.
    pub async fn close(&self) {
        self.pool.close().await;
    }

    /// Configuração utilizada pelo storage.
    pub fn config(&self) -> &SqliteStorageConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn in_memory_database_connects_and_pings() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        storage.ping().await.unwrap();
        assert!(storage.config().database_path.is_none());
        storage.close().await;
    }

    #[tokio::test]
    async fn file_database_creates_and_persists_table() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_hank.db");

        let config = SqliteStorageConfig::for_file(&db_path);
        let storage = SqliteStorage::connect(config).await.unwrap();

        storage
            .pool()
            .execute("CREATE TABLE test_items (id TEXT PRIMARY KEY, value TEXT NOT NULL);")
            .await
            .unwrap();

        storage
            .pool()
            .execute("INSERT INTO test_items (id, value) VALUES ('1', 'hello');")
            .await
            .unwrap();

        storage.close().await;
        assert!(db_path.exists());

        // Reabre o arquivo e verifica integridade
        let storage_reopened = SqliteStorage::connect(SqliteStorageConfig::for_file(&db_path))
            .await
            .unwrap();

        storage_reopened.ping().await.unwrap();
        storage_reopened.close().await;
    }

    #[tokio::test]
    async fn transaction_commits_successfully() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();

        storage
            .pool()
            .execute("CREATE TABLE kv (k TEXT PRIMARY KEY, v TEXT);")
            .await
            .unwrap();

        let mut tx = storage.begin().await.unwrap();
        tx.execute("INSERT INTO kv (k, v) VALUES ('key1', 'val1');")
            .await
            .unwrap();
        tx.commit().await.unwrap();

        storage.ping().await.unwrap();
        storage.close().await;
    }

    #[tokio::test]
    async fn transaction_rollback_reverts_state() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();

        storage
            .pool()
            .execute("CREATE TABLE kv (k TEXT PRIMARY KEY, v TEXT);")
            .await
            .unwrap();

        let mut tx = storage.begin().await.unwrap();
        tx.execute("INSERT INTO kv (k, v) VALUES ('temp_key', 'temp_val');")
            .await
            .unwrap();
        tx.rollback().await.unwrap();

        storage.ping().await.unwrap();
        storage.close().await;
    }

    #[tokio::test]
    async fn path_validation_rejects_traversal_and_control_chars() {
        assert!(validate_sqlite_path(Path::new("../unsafe.db")).is_err());
        assert!(validate_sqlite_path(Path::new("subdir/../../etc/passwd.db")).is_err());
        assert!(validate_sqlite_path(Path::new("safe/dir/valid.db")).is_ok());
    }
}
