//! Explicit Agent↔Skill bindings layered on top of project skill imports.
//!
//! An agent can only bind an active skill already enabled for its project.
//! Bindings retain the exact version, bounded load budget and audit metadata;
//! skill content is never copied into this aggregate and never grants policy.

use crate::agent_repo::SqliteAgentRepository;
use crate::event_bus::EventBus;
use crate::project_skills::{
    ProjectSkillBinding, ProjectSkillService, SqliteProjectSkillBindingRepository,
};
use crate::skill_loader::{SkillLoadBudget, MAX_SKILL_LOAD_TOKENS};
use crate::skill_repo::SqliteSkillRepository;
use agent_core::{
    Action, AgentId, AgentStatus, Capability, CapabilitySet, DomainError, ProjectId, Resource,
    SkillId, SkillScope, SkillStatus, ToolDefaultAction, TraceId,
};
use agent_protocol::events::{ApplicationEvent, EventKind};
use agent_protocol::ids::EventId;
use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};

const MAX_ACTOR_ID_BYTES: usize = 128;
const MAX_APPROVAL_ID_BYTES: usize = 128;
const MAX_AGENT_SKILL_BINDINGS: usize = 64;
const MAX_PRECEDENCE: u32 = 65_535;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSkillBinding {
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub skill_id: SkillId,
    pub current_version: String,
    pub previous_version: Option<String>,
    pub precedence: u32,
    pub max_tokens: usize,
    pub enabled: bool,
    pub actor_id: String,
    pub approval_id: Option<String>,
    pub trace_id: TraceId,
    pub revision: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSkillBindingPolicy {
    pub allow: bool,
    pub allowed_capabilities: CapabilitySet,
    pub denied_capabilities: CapabilitySet,
    pub max_bindings: usize,
    pub max_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSkillBindingRequest {
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub skill_id: SkillId,
    pub version: String,
    pub precedence: u32,
    pub actor_id: String,
    pub capability: Capability,
    pub policy: AgentSkillBindingPolicy,
    pub approval_id: Option<String>,
    pub trace_id: TraceId,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSkillMutationRequest {
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub skill_id: SkillId,
    pub actor_id: String,
    pub capability: Capability,
    pub policy: AgentSkillBindingPolicy,
    pub approval_id: Option<String>,
    pub trace_id: TraceId,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSkillBindingMutation {
    pub binding: AgentSkillBinding,
    pub changed: bool,
    pub event_id: Option<EventId>,
}

#[derive(Clone)]
pub struct SqliteAgentSkillBindingRepository {
    pool: Pool<Sqlite>,
}

impl SqliteAgentSkillBindingRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn get(
        &self,
        project_id: &ProjectId,
        agent_id: &AgentId,
        skill_id: &SkillId,
    ) -> Result<Option<AgentSkillBinding>, DomainError> {
        let row = sqlx::query(
            "SELECT project_id, agent_id, skill_id, current_version, previous_version, precedence, max_tokens, enabled, actor_id, approval_id, trace_id, revision, created_at, updated_at FROM agent_skill_bindings WHERE project_id = ? AND agent_id = ? AND skill_id = ?",
        )
        .bind(project_id.to_string())
        .bind(agent_id.to_string())
        .bind(skill_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| persistence_error("agent skill binding query", error))?;
        row.map(decode_binding).transpose()
    }

    pub async fn list(
        &self,
        project_id: &ProjectId,
        agent_id: &AgentId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AgentSkillBinding>, DomainError> {
        let rows = sqlx::query(
            "SELECT project_id, agent_id, skill_id, current_version, previous_version, precedence, max_tokens, enabled, actor_id, approval_id, trace_id, revision, created_at, updated_at FROM agent_skill_bindings WHERE project_id = ? AND agent_id = ? AND enabled = 1 ORDER BY precedence ASC, skill_id ASC LIMIT ? OFFSET ?",
        )
        .bind(project_id.to_string())
        .bind(agent_id.to_string())
        .bind(limit.min(MAX_AGENT_SKILL_BINDINGS) as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| persistence_error("agent skill binding list", error))?;
        rows.into_iter().map(decode_binding).collect()
    }

    async fn count_enabled(
        &self,
        project_id: &ProjectId,
        agent_id: &AgentId,
    ) -> Result<usize, DomainError> {
        let row = sqlx::query(
            "SELECT COUNT(*) AS count FROM agent_skill_bindings WHERE project_id = ? AND agent_id = ? AND enabled = 1",
        )
        .bind(project_id.to_string())
        .bind(agent_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|error| persistence_error("agent skill binding count", error))?;
        let count: i64 = row
            .try_get("count")
            .map_err(|error| persistence_error("agent skill binding count decode", error))?;
        usize::try_from(count).map_err(|_| {
            DomainError::InvariantViolation("invalid agent skill binding count".into())
        })
    }

    async fn insert(&self, binding: &AgentSkillBinding) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO agent_skill_bindings (project_id, agent_id, skill_id, current_version, previous_version, precedence, max_tokens, enabled, actor_id, approval_id, trace_id, revision, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(binding.project_id.to_string())
        .bind(binding.agent_id.to_string())
        .bind(binding.skill_id.to_string())
        .bind(&binding.current_version)
        .bind(binding.previous_version.as_deref())
        .bind(i64::from(binding.precedence))
        .bind(i64::try_from(binding.max_tokens).map_err(|_| {
            DomainError::Validation("agent skill token budget is invalid".into())
        })?)
        .bind(if binding.enabled { 1_i64 } else { 0_i64 })
        .bind(&binding.actor_id)
        .bind(binding.approval_id.as_deref())
        .bind(binding.trace_id.to_string())
        .bind(revision_i64(binding.revision)?)
        .bind(binding.created_at.to_rfc3339())
        .bind(binding.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|error| persistence_error("agent skill binding insert", error))?;
        Ok(())
    }

    async fn update(
        &self,
        binding: &AgentSkillBinding,
        expected_revision: u64,
    ) -> Result<(), DomainError> {
        let result = sqlx::query(
            "UPDATE agent_skill_bindings SET current_version = ?, previous_version = ?, precedence = ?, max_tokens = ?, enabled = ?, actor_id = ?, approval_id = ?, trace_id = ?, revision = ?, updated_at = ? WHERE project_id = ? AND agent_id = ? AND skill_id = ? AND revision = ?",
        )
        .bind(&binding.current_version)
        .bind(binding.previous_version.as_deref())
        .bind(i64::from(binding.precedence))
        .bind(i64::try_from(binding.max_tokens).map_err(|_| {
            DomainError::Validation("agent skill token budget is invalid".into())
        })?)
        .bind(if binding.enabled { 1_i64 } else { 0_i64 })
        .bind(&binding.actor_id)
        .bind(binding.approval_id.as_deref())
        .bind(binding.trace_id.to_string())
        .bind(revision_i64(binding.revision)?)
        .bind(binding.updated_at.to_rfc3339())
        .bind(binding.project_id.to_string())
        .bind(binding.agent_id.to_string())
        .bind(binding.skill_id.to_string())
        .bind(revision_i64(expected_revision)?)
        .execute(&self.pool)
        .await
        .map_err(|error| persistence_error("agent skill binding update", error))?;
        if result.rows_affected() == 0 {
            return Err(DomainError::ConcurrencyConflict {
                expected: expected_revision.to_string(),
                actual: binding.revision.to_string(),
            });
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct AgentSkillService {
    agents: SqliteAgentRepository,
    skills: SqliteSkillRepository,
    project_bindings: SqliteProjectSkillBindingRepository,
    bindings: SqliteAgentSkillBindingRepository,
    project_service: ProjectSkillService,
    event_bus: Option<EventBus<ApplicationEvent>>,
}

impl AgentSkillService {
    pub fn new(
        agents: SqliteAgentRepository,
        skills: SqliteSkillRepository,
        project_bindings: SqliteProjectSkillBindingRepository,
        bindings: SqliteAgentSkillBindingRepository,
    ) -> Self {
        Self {
            project_service: ProjectSkillService::new(skills.clone(), project_bindings.clone()),
            agents,
            skills,
            project_bindings,
            bindings,
            event_bus: None,
        }
    }

    pub fn with_event_bus(mut self, event_bus: EventBus<ApplicationEvent>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    pub async fn bind(
        &self,
        request: AgentSkillBindingRequest,
    ) -> Result<AgentSkillBindingMutation, DomainError> {
        validate_binding_request(&request)?;
        let agent = self
            .agents
            .get(&request.project_id, &request.agent_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("agent not found in project".into()))?;
        ensure_agent_active(&agent.status)?;
        let project_binding = self
            .project_bindings
            .get(&request.project_id, &request.skill_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("project skill is not bound".into()))?;
        if !project_binding.enabled {
            return Err(DomainError::NotFound(
                "project skill binding is disabled".into(),
            ));
        }
        if project_binding.current_version != request.version {
            return Err(DomainError::NotFound(
                "agent skill version is not the enabled project version".into(),
            ));
        }
        let record = self
            .resolve_record(
                &request.project_id,
                &request.skill_id,
                &project_binding,
                None,
            )
            .await?;
        if record.skill.status != SkillStatus::Active
            || record.skill.manifest.version != request.version
        {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", record.skill.status),
                to: "agent-bound".into(),
            });
        }
        validate_agent_policy(
            &agent,
            &record.skill.manifest.capabilities,
            record.skill.manifest.budget.max_tokens,
            &request,
        )?;
        if record.skill.manifest.policy.requires_approval && request.approval_id.is_none() {
            return Err(DomainError::PermissionDenied {
                capability: "skill.bind".into(),
                reason: "skill binding requires explicit approval".into(),
            });
        }

        if let Some(existing) = self
            .bindings
            .get(&request.project_id, &request.agent_id, &request.skill_id)
            .await?
        {
            check_expected_revision(request.expected_revision, existing.revision)?;
            if existing.current_version == request.version
                && existing.precedence == request.precedence
                && existing.max_tokens == request.policy.max_tokens
                && existing.enabled
            {
                return Ok(AgentSkillBindingMutation {
                    binding: existing,
                    changed: false,
                    event_id: None,
                });
            }
            let next_revision = next_revision(existing.revision)?;
            let binding = AgentSkillBinding {
                project_id: request.project_id,
                agent_id: request.agent_id,
                skill_id: request.skill_id,
                current_version: request.version,
                previous_version: (existing.current_version != project_binding.current_version)
                    .then_some(existing.current_version),
                precedence: request.precedence,
                max_tokens: request.policy.max_tokens,
                enabled: true,
                actor_id: request.actor_id,
                approval_id: request.approval_id,
                trace_id: request.trace_id,
                revision: next_revision,
                created_at: existing.created_at,
                updated_at: Utc::now(),
            };
            self.bindings.update(&binding, existing.revision).await?;
            let event_id = self.publish_event("agent-bind", &binding);
            return Ok(AgentSkillBindingMutation {
                binding,
                changed: true,
                event_id,
            });
        }

        if self
            .bindings
            .count_enabled(&request.project_id, &request.agent_id)
            .await?
            >= request.policy.max_bindings
        {
            return Err(DomainError::BudgetExceeded {
                budget_type: "agent_skill_bindings".into(),
                limit: request.policy.max_bindings.to_string(),
                used: request.policy.max_bindings.to_string(),
            });
        }
        if let Some(expected) = request.expected_revision {
            return Err(DomainError::ConcurrencyConflict {
                expected: expected.to_string(),
                actual: "missing".into(),
            });
        }
        let now = Utc::now();
        let binding = AgentSkillBinding {
            project_id: request.project_id,
            agent_id: request.agent_id,
            skill_id: request.skill_id,
            current_version: request.version,
            previous_version: None,
            precedence: request.precedence,
            max_tokens: request.policy.max_tokens,
            enabled: true,
            actor_id: request.actor_id,
            approval_id: request.approval_id,
            trace_id: request.trace_id,
            revision: 1,
            created_at: now,
            updated_at: now,
        };
        self.bindings.insert(&binding).await?;
        let event_id = self.publish_event("agent-bind", &binding);
        Ok(AgentSkillBindingMutation {
            binding,
            changed: true,
            event_id,
        })
    }

    pub async fn list(
        &self,
        project_id: &ProjectId,
        agent_id: &AgentId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AgentSkillBinding>, DomainError> {
        self.bindings
            .list(project_id, agent_id, limit, offset)
            .await
    }

    pub async fn disable(
        &self,
        request: AgentSkillMutationRequest,
    ) -> Result<AgentSkillBindingMutation, DomainError> {
        self.mutate_disabled(request, "disable").await
    }

    pub async fn rollback(
        &self,
        request: AgentSkillMutationRequest,
    ) -> Result<AgentSkillBindingMutation, DomainError> {
        self.mutate_disabled(request, "rollback").await
    }

    pub async fn load_bound(
        &self,
        project_id: ProjectId,
        agent_id: AgentId,
        skill_id: SkillId,
    ) -> Result<crate::skill_loader::LoadedSkill, DomainError> {
        let agent = self
            .agents
            .get(&project_id, &agent_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("agent not found in project".into()))?;
        ensure_agent_active(&agent.status)?;
        let binding = self
            .bindings
            .get(&project_id, &agent_id, &skill_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("agent skill is not bound".into()))?;
        if !binding.enabled {
            return Err(DomainError::NotFound(
                "agent skill binding is disabled".into(),
            ));
        }
        let project_binding = self
            .project_bindings
            .get(&project_id, &skill_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("project skill is not bound".into()))?;
        if !project_binding.enabled || project_binding.current_version != binding.current_version {
            return Err(DomainError::NotFound(
                "agent skill pin is no longer enabled by the project".into(),
            ));
        }
        let record = self
            .resolve_record(
                &project_id,
                &skill_id,
                &project_binding,
                Some(&binding.current_version),
            )
            .await?;
        if record.skill.status != SkillStatus::Active
            || record.skill.manifest.version != binding.current_version
        {
            return Err(DomainError::NotFound(
                "agent skill pin is no longer active".into(),
            ));
        }
        validate_current_agent_capabilities(&agent, &record.skill.manifest.capabilities)?;
        self.project_service
            .load_bound_with_budget(
                project_id,
                agent_id,
                skill_id,
                SkillLoadBudget {
                    max_tokens: binding.max_tokens,
                    ..SkillLoadBudget::default()
                },
            )
            .await
    }

    async fn mutate_disabled(
        &self,
        request: AgentSkillMutationRequest,
        action: &str,
    ) -> Result<AgentSkillBindingMutation, DomainError> {
        validate_mutation_request(&request)?;
        self.agents
            .get(&request.project_id, &request.agent_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("agent not found in project".into()))?;
        let existing = self
            .bindings
            .get(&request.project_id, &request.agent_id, &request.skill_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("agent skill binding not found".into()))?;
        check_expected_revision(request.expected_revision, existing.revision)?;
        if !existing.enabled {
            return Ok(AgentSkillBindingMutation {
                binding: existing,
                changed: false,
                event_id: None,
            });
        }
        let mut binding = existing.clone();
        binding.enabled = false;
        binding.actor_id = request.actor_id;
        binding.approval_id = request.approval_id;
        binding.trace_id = request.trace_id;
        binding.revision = next_revision(existing.revision)?;
        binding.updated_at = Utc::now();
        self.bindings.update(&binding, existing.revision).await?;
        let event_id = self.publish_event(action, &binding);
        Ok(AgentSkillBindingMutation {
            binding,
            changed: true,
            event_id,
        })
    }

    async fn resolve_record(
        &self,
        project_id: &ProjectId,
        skill_id: &SkillId,
        binding: &ProjectSkillBinding,
        version: Option<&str>,
    ) -> Result<crate::skill_repo::SkillRecord, DomainError> {
        let record = match binding.scope {
            SkillScope::Project => {
                if let Some(version) = version {
                    self.skills
                        .get_version(binding.scope, Some(project_id), skill_id, version)
                        .await?
                } else {
                    self.skills
                        .get(binding.scope, Some(project_id), skill_id)
                        .await?
                }
            }
            SkillScope::Global => {
                if let Some(version) = version {
                    self.skills
                        .get_version(binding.scope, None, skill_id, version)
                        .await?
                } else {
                    self.skills.get(binding.scope, None, skill_id).await?
                }
            }
        };
        record.ok_or_else(|| DomainError::NotFound("project skill source not found".into()))
    }

    fn publish_event(&self, action: &str, binding: &AgentSkillBinding) -> Option<EventId> {
        let bus = self.event_bus.as_ref()?;
        let event_id = EventId::new();
        let event = ApplicationEvent {
            schema_version: 1,
            event_id,
            event_type: EventKind::SkillBindingChanged,
            project_id: binding.project_id,
            aggregate_id: binding.skill_id.to_string(),
            agent_id: Some(binding.agent_id),
            session_id: None,
            occurred_at: Utc::now(),
            sequence: binding.revision,
            payload: serde_json::json!({
                "action": action,
                "skill_id": binding.skill_id,
                "version": binding.current_version,
                "precedence": binding.precedence,
                "max_tokens": binding.max_tokens,
                "enabled": binding.enabled,
                "actor_id": binding.actor_id,
                "approval_id": binding.approval_id,
                "trace_id": binding.trace_id,
                "revision": binding.revision,
            })
            .to_string(),
        };
        let _ = bus.publish(event);
        Some(event_id)
    }
}

fn validate_binding_request(request: &AgentSkillBindingRequest) -> Result<(), DomainError> {
    validate_mutation_context(
        request.project_id,
        request.agent_id,
        &request.actor_id,
        &request.capability,
        &request.policy,
        request.trace_id,
    )?;
    Version::parse(&request.version)
        .map_err(|_| DomainError::Validation("skill version is invalid".into()))?;
    if request.precedence > MAX_PRECEDENCE {
        return Err(DomainError::Validation(
            "agent skill precedence is out of bounds".into(),
        ));
    }
    validate_optional_text(
        request.approval_id.as_deref(),
        MAX_APPROVAL_ID_BYTES,
        "approval_id",
    )
}

fn validate_mutation_request(request: &AgentSkillMutationRequest) -> Result<(), DomainError> {
    validate_mutation_context(
        request.project_id,
        request.agent_id,
        &request.actor_id,
        &request.capability,
        &request.policy,
        request.trace_id,
    )?;
    validate_optional_text(
        request.approval_id.as_deref(),
        MAX_APPROVAL_ID_BYTES,
        "approval_id",
    )
}

fn validate_mutation_context(
    project_id: ProjectId,
    agent_id: AgentId,
    actor_id: &str,
    capability: &Capability,
    policy: &AgentSkillBindingPolicy,
    trace_id: TraceId,
) -> Result<(), DomainError> {
    if project_id.as_uuid().is_nil() || agent_id.as_uuid().is_nil() || trace_id.as_uuid().is_nil() {
        return Err(DomainError::Validation(
            "agent skill binding identity is invalid".into(),
        ));
    }
    validate_optional_text(Some(actor_id), MAX_ACTOR_ID_BYTES, "actor_id")?;
    if !policy.allow {
        return Err(DomainError::PermissionDenied {
            capability: capability.to_string(),
            reason: "agent skill binding policy denied the operation".into(),
        });
    }
    let required =
        Capability::new(Resource::Skill, Action::Configure).with_scope(project_id.to_string());
    if capability != &required || !policy.allowed_capabilities.contains(&required) {
        return Err(DomainError::CapabilityUnavailable(
            "skill configure capability is required for agent binding".into(),
        ));
    }
    if policy.max_bindings == 0 || policy.max_bindings > MAX_AGENT_SKILL_BINDINGS {
        return Err(DomainError::Validation(
            "invalid agent skill binding budget".into(),
        ));
    }
    if policy.max_tokens == 0 || policy.max_tokens > MAX_SKILL_LOAD_TOKENS {
        return Err(DomainError::Validation(
            "invalid agent skill token budget".into(),
        ));
    }
    Ok(())
}

fn validate_agent_policy(
    agent: &agent_core::Agent,
    declared: &[Capability],
    skill_max_tokens: u64,
    request: &AgentSkillBindingRequest,
) -> Result<(), DomainError> {
    if request.policy.max_tokens as u64 > skill_max_tokens {
        return Err(DomainError::BudgetExceeded {
            budget_type: "skill_tokens".into(),
            limit: skill_max_tokens.to_string(),
            used: request.policy.max_tokens.to_string(),
        });
    }
    if let Some(max_tokens) = agent.policy.budget.max_tokens_per_request {
        if request.policy.max_tokens as u64 > max_tokens {
            return Err(DomainError::BudgetExceeded {
                budget_type: "agent_skill_tokens".into(),
                limit: max_tokens.to_string(),
                used: request.policy.max_tokens.to_string(),
            });
        }
    }
    validate_current_agent_capabilities(agent, declared)?;
    for capability in declared {
        if request.policy.denied_capabilities.contains(capability)
            || agent.policy.tools.denied.contains(capability)
        {
            return Err(DomainError::CapabilityUnavailable(
                "skill capability is denied by agent policy".into(),
            ));
        }
        if !request.policy.allowed_capabilities.contains(capability) {
            return Err(DomainError::CapabilityUnavailable(
                "skill capability is not allowed by binding policy".into(),
            ));
        }
        if (!agent.policy.tools.allowed.capabilities.is_empty()
            && !agent.policy.tools.allowed.contains(capability))
            || (agent.policy.tools.default_action == ToolDefaultAction::Deny
                && !agent.policy.tools.allowed.contains(capability))
        {
            return Err(DomainError::CapabilityUnavailable(
                "skill capability is not allowed by agent policy".into(),
            ));
        }
    }
    Ok(())
}

fn validate_current_agent_capabilities(
    agent: &agent_core::Agent,
    declared: &[Capability],
) -> Result<(), DomainError> {
    for capability in declared {
        if agent.policy.tools.denied.contains(capability)
            || (!agent.policy.tools.allowed.capabilities.is_empty()
                && !agent.policy.tools.allowed.contains(capability))
            || (agent.policy.tools.default_action == ToolDefaultAction::Deny
                && !agent.policy.tools.allowed.contains(capability))
        {
            return Err(DomainError::CapabilityUnavailable(
                "skill capability is not allowed by current agent policy".into(),
            ));
        }
    }
    Ok(())
}

fn ensure_agent_active(status: &AgentStatus) -> Result<(), DomainError> {
    if *status != AgentStatus::Active {
        return Err(DomainError::InvalidStateTransition {
            from: format!("{status:?}"),
            to: "agent-bound".into(),
        });
    }
    Ok(())
}

fn validate_optional_text(
    value: Option<&str>,
    max_bytes: usize,
    field: &str,
) -> Result<(), DomainError> {
    if let Some(value) = value {
        if value.trim().is_empty()
            || value.len() > max_bytes
            || value.chars().any(char::is_control)
            || value.contains("..")
        {
            return Err(DomainError::Validation(format!("invalid {field}")));
        }
    }
    Ok(())
}

fn check_expected_revision(expected: Option<u64>, actual: u64) -> Result<(), DomainError> {
    if let Some(expected) = expected {
        if expected != actual {
            return Err(DomainError::ConcurrencyConflict {
                expected: expected.to_string(),
                actual: actual.to_string(),
            });
        }
    }
    Ok(())
}

fn next_revision(revision: u64) -> Result<u64, DomainError> {
    revision.checked_add(1).ok_or_else(|| {
        DomainError::InvariantViolation("agent skill binding revision overflow".into())
    })
}

fn revision_i64(revision: u64) -> Result<i64, DomainError> {
    i64::try_from(revision)
        .map_err(|_| DomainError::Validation("agent skill binding revision is invalid".into()))
}

fn decode_binding(row: sqlx::sqlite::SqliteRow) -> Result<AgentSkillBinding, DomainError> {
    let project_id = row_string(&row, "project_id")?
        .parse()
        .map_err(|_| DomainError::Validation("invalid agent skill binding project id".into()))?;
    let agent_id = row_string(&row, "agent_id")?
        .parse()
        .map_err(|_| DomainError::Validation("invalid agent skill binding agent id".into()))?;
    let skill_id = row_string(&row, "skill_id")?
        .parse()
        .map_err(|_| DomainError::Validation("invalid agent skill binding skill id".into()))?;
    let precedence = u32::try_from(row_i64(&row, "precedence")?)
        .map_err(|_| DomainError::Validation("invalid agent skill precedence".into()))?;
    let max_tokens = usize::try_from(row_i64(&row, "max_tokens")?)
        .map_err(|_| DomainError::Validation("invalid agent skill token budget".into()))?;
    let revision = u64::try_from(row_i64(&row, "revision")?)
        .map_err(|_| DomainError::Validation("invalid agent skill revision".into()))?;
    let enabled = row_i64(&row, "enabled")?;
    if enabled != 0 && enabled != 1 {
        return Err(DomainError::Validation(
            "invalid agent skill enabled state".into(),
        ));
    }
    Ok(AgentSkillBinding {
        project_id,
        agent_id,
        skill_id,
        current_version: row_string(&row, "current_version")?,
        previous_version: row_optional_string(&row, "previous_version")?,
        precedence,
        max_tokens,
        enabled: enabled == 1,
        actor_id: row_string(&row, "actor_id")?,
        approval_id: row_optional_string(&row, "approval_id")?,
        trace_id: row_string(&row, "trace_id")?
            .parse()
            .map_err(|_| DomainError::Validation("invalid agent skill trace id".into()))?,
        revision,
        created_at: parse_time(&row_string(&row, "created_at")?)?,
        updated_at: parse_time(&row_string(&row, "updated_at")?)?,
    })
}

fn row_string(row: &sqlx::sqlite::SqliteRow, column: &str) -> Result<String, DomainError> {
    row.try_get(column)
        .map_err(|error| persistence_error("agent skill text decode", error))
}

fn row_optional_string(
    row: &sqlx::sqlite::SqliteRow,
    column: &str,
) -> Result<Option<String>, DomainError> {
    row.try_get(column)
        .map_err(|error| persistence_error("agent skill optional text decode", error))
}

fn row_i64(row: &sqlx::sqlite::SqliteRow, column: &str) -> Result<i64, DomainError> {
    row.try_get(column)
        .map_err(|error| persistence_error("agent skill integer decode", error))
}

fn parse_time(value: &str) -> Result<DateTime<Utc>, DomainError> {
    DateTime::parse_from_rfc3339(value)
        .map(|time| time.with_timezone(&Utc))
        .map_err(|_| DomainError::Validation("invalid agent skill timestamp".into()))
}

fn persistence_error(context: &str, error: sqlx::Error) -> DomainError {
    DomainError::InvariantViolation(format!("{context}: {error}"))
}
