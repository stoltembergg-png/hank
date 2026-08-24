-- Explicit provenance columns keep the immutable version graph queryable
-- without trusting mutable head state or reparsing untrusted content.
ALTER TABLE skill_versions ADD COLUMN parent_version TEXT;
ALTER TABLE skill_versions ADD COLUMN compatibility TEXT NOT NULL DEFAULT 'initial'
    CHECK (compatibility IN ('initial', 'compatible', 'incompatible'));
ALTER TABLE skill_versions ADD COLUMN hash_algorithm TEXT NOT NULL DEFAULT 'legacy'
    CHECK (hash_algorithm IN ('legacy', 'sha256-v1'));

-- Lifecycle state belongs to the immutable version reference, not only to the
-- mutable head. This keeps an exact project/Agent pin loadable after head
-- rollback without mutating the artifact payload.
CREATE TABLE skill_version_states (
    namespace TEXT NOT NULL,
    skill_id TEXT NOT NULL,
    manifest_version TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('draft', 'testing', 'active', 'deprecated', 'archived', 'blocked')),
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    updated_at TEXT NOT NULL,
    PRIMARY KEY (namespace, skill_id, manifest_version),
    FOREIGN KEY (namespace, skill_id, manifest_version)
        REFERENCES skill_versions(namespace, skill_id, manifest_version)
        ON DELETE CASCADE
);

-- Existing rows predate explicit version states. Preserve the state of the
-- current head; historical rows remain conservatively draft until promoted
-- again through an explicit operation.
INSERT INTO skill_version_states (namespace, skill_id, manifest_version, status, revision, updated_at)
SELECT sv.namespace,
       sv.skill_id,
       sv.manifest_version,
       COALESCE(CASE WHEN sh.current_version = sv.manifest_version THEN sh.status END, 'draft'),
       1,
       sv.created_at
FROM skill_versions AS sv
LEFT JOIN skill_heads AS sh
  ON sh.namespace = sv.namespace AND sh.skill_id = sv.skill_id;

CREATE INDEX skill_versions_parent_idx
    ON skill_versions(namespace, skill_id, parent_version, manifest_version);

CREATE INDEX skill_version_states_status_idx
    ON skill_version_states(namespace, skill_id, status, updated_at DESC);
