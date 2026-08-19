# SQLite Storage (PR-025)

## Overview

The `SqliteStorage` engine in `crates/agent-runtime` provides transactional local persistence for application metadata.

## Architectural Boundaries

- **Pure Domain Separation (AI-001, AI-003, D-001):** `agent-core` never imports `sqlx` or concrete storage. Persistence logic is encapsulated in `agent-runtime` and consumed via domain ports.
- **Frontend Isolation:** Frontend and UI components are strictly forbidden from accessing SQLite directly or importing SQLx.

## Storage Configuration & Pragmas

- **Connection Pool:** Managed via `sqlx::SqlitePool` with bounded connections (`max_connections`) and configurable `busy_timeout`.
- **Journal Mode (WAL):** Write-Ahead Logging (`PRAGMA journal_mode = WAL;`) is enabled for file-based databases to support concurrent readers without blocking writes.
- **Foreign Keys:** Enforced via `PRAGMA foreign_keys = ON;` on every acquired connection.
- **Synchronous Mode:** Configured to `NORMAL` for high performance with durability under WAL.

## Security & Path Validation

- **Path Traversal Protection:** `validate_sqlite_path` rejects paths containing `..` or control characters.
- **No Plaintext Secrets:** Sensitive credentials and secrets are managed by the encrypted secret storage and never stored plaintext in SQLite.

## Usage

```rust
use agent_runtime::sqlite::{SqliteStorage, SqliteStorageConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // In-memory database for testing
    let storage = SqliteStorage::connect_in_memory().await?;
    storage.ping().await?;

    // File-based database
    let config = SqliteStorageConfig::for_file("data/hank.db");
    let file_storage = SqliteStorage::connect(config).await?;

    // Transaction management
    let mut tx = file_storage.begin().await?;
    // ... perform queries ...
    tx.commit().await?;

    Ok(())
}
```
