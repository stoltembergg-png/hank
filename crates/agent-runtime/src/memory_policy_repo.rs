#![allow(clippy::manual_async_fn)]

use agent_core::{DomainError, MemoryPolicy, MemoryPolicyEntry, MemoryPolicyLayer, ProjectId};
use agent_protocol::AgentId;
use sqlx::{Pool, Row, Sqlite};

#[derive(Clone)]
pub struct SqliteMemoryPolicyRepository {
    pool: Pool<Sqlite>,
}

impl SqliteMemoryPolicyRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn latest(
        &self,
        project_id: &ProjectId,
        agent_id: &AgentId,
        layer: MemoryPolicyLayer,
    ) -> Result<Option<MemoryPolicyEntry>, DomainError> {
        let row = sqlx::query(
            "SELECT policy_json FROM memory_policies WHERE project_id = ? AND agent_id = ? AND layer = ? AND active = 1 ORDER BY version DESC LIMIT 1",
        )
        .bind(project_id.to_string())
        .bind(agent_id.to_string())
        .bind(layer_name(layer))
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| DomainError::InvariantViolation(format!("memory policy query failed: {error}")))?;
        row.map(|row| {
            let json: String = row.try_get("policy_json").map_err(|error| {
                DomainError::InvariantViolation(format!("memory policy row failed: {error}"))
            })?;
            let policy: MemoryPolicy = serde_json::from_str(&json).map_err(|error| {
                DomainError::Validation(format!("invalid stored memory policy: {error}"))
            })?;
            Ok(MemoryPolicyEntry { layer, policy })
        })
        .transpose()
    }

    pub async fn save(
        &self,
        entry: &MemoryPolicyEntry,
        expected_version: Option<u64>,
    ) -> Result<(), DomainError> {
        entry.policy.validate().map_err(DomainError::Validation)?;
        let current: Option<i64> = sqlx::query_scalar(
            "SELECT MAX(version) FROM memory_policies WHERE project_id = ? AND agent_id = ? AND layer = ?",
        )
        .bind(entry.policy.project_id.to_string())
        .bind(entry.policy.agent_id.to_string())
        .bind(layer_name(entry.layer))
        .fetch_one(&self.pool)
        .await
        .map_err(|error| DomainError::InvariantViolation(format!("memory policy version query failed: {error}")))?;
        let current = current.map(|version| version as u64);
        if current != expected_version {
            return Err(DomainError::ConcurrencyConflict {
                expected: expected_version.map_or_else(|| "none".into(), |value| value.to_string()),
                actual: current.map_or_else(|| "none".into(), |value| value.to_string()),
            });
        }
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO memory_policies (project_id, agent_id, layer, version, policy_json, active, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 1, ?, ?)",
        )
        .bind(entry.policy.project_id.to_string())
        .bind(entry.policy.agent_id.to_string())
        .bind(layer_name(entry.layer))
        .bind(entry.policy.version as i64)
        .bind(serde_json::to_string(&entry.policy)?)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|error| DomainError::InvariantViolation(format!("memory policy save failed: {error}")))?;
        Ok(())
    }

    pub async fn rollback(
        &self,
        project_id: &ProjectId,
        agent_id: &AgentId,
        layer: MemoryPolicyLayer,
        target_version: u64,
        expected_version: u64,
    ) -> Result<MemoryPolicyEntry, DomainError> {
        let row = sqlx::query(
            "SELECT policy_json FROM memory_policies WHERE project_id = ? AND agent_id = ? AND layer = ? AND version = ?",
        )
        .bind(project_id.to_string())
        .bind(agent_id.to_string())
        .bind(layer_name(layer))
        .bind(target_version as i64)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| DomainError::InvariantViolation(format!("memory policy rollback query failed: {error}")))?
        .ok_or_else(|| DomainError::NotFound("memory policy version not found".into()))?;
        let json: String = row.try_get("policy_json").map_err(|error| {
            DomainError::InvariantViolation(format!("memory policy rollback row failed: {error}"))
        })?;
        let mut policy: MemoryPolicy = serde_json::from_str(&json).map_err(|error| {
            DomainError::Validation(format!("invalid stored memory policy: {error}"))
        })?;
        policy.version = expected_version.saturating_add(1);
        let entry = MemoryPolicyEntry { layer, policy };
        self.save(&entry, Some(expected_version)).await?;
        Ok(entry)
    }
}

fn layer_name(layer: MemoryPolicyLayer) -> &'static str {
    match layer {
        MemoryPolicyLayer::System => "system",
        MemoryPolicyLayer::Security => "security",
        MemoryPolicyLayer::Project => "project",
        MemoryPolicyLayer::Agent => "agent",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        agent_repo::SqliteAgentRepository, migrations::run_migrations, sqlite::SqliteStorage,
    };
    use agent_core::{MemoryApprovalMode, MemoryType};

    async fn fixture() -> (SqliteMemoryPolicyRepository, ProjectId, AgentId) {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();
        let project_id = ProjectId::new();
        sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Policy project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
            .bind(project_id.to_string()).execute(storage.pool()).await.unwrap();
        let agent = agent_core::Agent::new(project_id, "policy-agent".into(), Default::default());
        SqliteAgentRepository::new(storage.pool().clone())
            .save(&agent)
            .await
            .unwrap();
        (
            SqliteMemoryPolicyRepository::new(storage.pool().clone()),
            project_id,
            agent.id,
        )
    }

    fn entry(project_id: ProjectId, agent_id: AgentId, version: u64) -> MemoryPolicyEntry {
        MemoryPolicyEntry {
            layer: MemoryPolicyLayer::Agent,
            policy: MemoryPolicy {
                schema_version: agent_core::MEMORY_POLICY_SCHEMA_VERSION,
                project_id,
                agent_id,
                version,
                read: true,
                write: true,
                learn: false,
                allowed_types: vec![MemoryType::Semantic],
                max_tokens: 100,
                max_cost_micros: 100,
                retention_days: 30,
                approval_mode: MemoryApprovalMode::CandidateOnly,
                autonomy_level: agent_protocol::AutonomyLevel::Assisted,
                allow_rollback: true,
            },
        }
    }

    #[tokio::test]
    async fn policy_repository_is_versioned_project_agent_scoped_and_rolls_back() {
        let (repo, project, agent) = fixture().await;
        let first = entry(project, agent, 1);
        repo.save(&first, None).await.unwrap();
        assert_eq!(
            repo.latest(&project, &agent, MemoryPolicyLayer::Agent)
                .await
                .unwrap()
                .unwrap()
                .policy
                .version,
            1
        );
        let mut second = entry(project, agent, 2);
        second.policy.write = false;
        repo.save(&second, Some(1)).await.unwrap();
        assert!(
            !repo
                .latest(&project, &agent, MemoryPolicyLayer::Agent)
                .await
                .unwrap()
                .unwrap()
                .policy
                .write
        );
        let rolled = repo
            .rollback(&project, &agent, MemoryPolicyLayer::Agent, 1, 2)
            .await
            .unwrap();
        assert_eq!(rolled.policy.version, 3);
        assert!(
            repo.latest(&project, &agent, MemoryPolicyLayer::Agent)
                .await
                .unwrap()
                .unwrap()
                .policy
                .write
        );
        assert!(repo
            .latest(&ProjectId::new(), &agent, MemoryPolicyLayer::Agent)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn policy_repository_rejects_stale_updates_and_unknown_fields() {
        let (repo, project, agent) = fixture().await;
        let first = entry(project, agent, 1);
        repo.save(&first, None).await.unwrap();
        assert!(matches!(
            repo.save(&entry(project, agent, 2), None).await,
            Err(DomainError::ConcurrencyConflict { .. })
        ));
        let mut invalid = serde_json::to_value(first.policy).unwrap();
        invalid["model_override"] = serde_json::json!("allow");
        assert!(serde_json::from_value::<MemoryPolicy>(invalid).is_err());
    }
}
