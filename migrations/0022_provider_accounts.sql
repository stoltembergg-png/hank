-- Migration 0022: durable provider account metadata without secret material

CREATE TABLE IF NOT EXISTS provider_accounts (
    project_id TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    account_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('connected', 'pending', 'revoked', 'unavailable', 'error')),
    credential_ref TEXT,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (project_id, provider_id, account_id),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_provider_accounts_project_state
    ON provider_accounts(project_id, state, updated_at);
