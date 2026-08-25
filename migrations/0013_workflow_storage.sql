-- Migration 0013: versioned workflow definitions, nodes and edges

CREATE TABLE IF NOT EXISTS workflow_definitions (
    workflow_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    schema_version INTEGER NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    policy_ref TEXT NOT NULL,
    metadata TEXT NOT NULL,
    PRIMARY KEY (workflow_id, version),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_workflow_definitions_project_version
    ON workflow_definitions(project_id, version);
CREATE INDEX IF NOT EXISTS idx_workflow_definitions_project_status
    ON workflow_definitions(project_id, status);

CREATE TABLE IF NOT EXISTS workflow_nodes (
    workflow_id TEXT NOT NULL,
    workflow_version INTEGER NOT NULL,
    node_id TEXT NOT NULL,
    schema_version INTEGER NOT NULL,
    node_type TEXT NOT NULL,
    input_schema TEXT NOT NULL,
    output_schema TEXT NOT NULL,
    timeout_ms INTEGER NOT NULL,
    retry_max_attempts INTEGER NOT NULL,
    cancel_policy TEXT NOT NULL,
    required_capabilities TEXT NOT NULL,
    PRIMARY KEY (workflow_id, workflow_version, node_id),
    FOREIGN KEY (workflow_id, workflow_version)
        REFERENCES workflow_definitions(workflow_id, version)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS workflow_edges (
    workflow_id TEXT NOT NULL,
    workflow_version INTEGER NOT NULL,
    edge_id TEXT NOT NULL,
    source_node TEXT NOT NULL,
    source_port TEXT NOT NULL,
    target_node TEXT NOT NULL,
    target_port TEXT NOT NULL,
    condition TEXT,
    ordering INTEGER NOT NULL,
    PRIMARY KEY (workflow_id, workflow_version, edge_id),
    FOREIGN KEY (workflow_id, workflow_version)
        REFERENCES workflow_definitions(workflow_id, version)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_workflow_nodes_version
    ON workflow_nodes(workflow_id, workflow_version);
CREATE INDEX IF NOT EXISTS idx_workflow_edges_version_order
    ON workflow_edges(workflow_id, workflow_version, ordering);
