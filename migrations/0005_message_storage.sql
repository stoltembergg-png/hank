ALTER TABLE messages ADD COLUMN schema_version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE messages ADD COLUMN provenance TEXT NOT NULL DEFAULT 'user';
ALTER TABLE messages ADD COLUMN status TEXT NOT NULL DEFAULT 'draft';
ALTER TABLE messages ADD COLUMN correlation_id TEXT NOT NULL DEFAULT 'legacy-correlation';
ALTER TABLE messages ADD COLUMN sequence INTEGER NOT NULL DEFAULT 0;
ALTER TABLE messages ADD COLUMN generation INTEGER NOT NULL DEFAULT 1;
ALTER TABLE messages ADD COLUMN parts TEXT NOT NULL DEFAULT '[]';

CREATE UNIQUE INDEX IF NOT EXISTS idx_messages_session_generation_sequence
    ON messages(session_id, generation, sequence);