//! Project-owned skill bindings.
//!
//! A binding is the explicit project boundary between a persisted Skill and
//! the loader. It stores only identifiers, policy metadata and provenance;
//! Skill content remains in the Skill repository and is never copied here.

use crate::event_bus::EventBus;
use crate::skill_loader::{SkillGlobalImport, SkillLoadBudget, SkillLoadPolicy, SkillLoadRequest};
use crate::skill_repo::{SkillRecord, SqliteSkillRepository};
use agent_core::{
    Action, AgentId, Capability, CapabilitySet, DomainError, ProjectId, Resource, SkillId,
    SkillScope, SkillSourceKind, SkillStatus, TraceId,
};
use agent_protocol::events::{ApplicationEvent, EventKind};
use agent_protocol::ids::EventId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};

const MAX_ACTOR_ID_BYTES: usize = 128;
const MAX_APPROVAL_ID_BYTES: usize = 128;
const MAX_IMPORT_REFERENCE_BYTES: usize = 256;
const MAX_BINDINGS_PER_PROJECT: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectSkillBinding {
    pub project_id: ProjectId,
    pub skill_id: SkillId,
    pub scope: SkillScope,
    pub current_version: String,
    pub previous_version: Option<String>,
    pub import_reference: Option<String>,
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
pub struct ProjectSkillBindingPolicy {
    pub allow: bool,
    pub allowed_capabilities: CapabilitySet,
    pub max_bindings: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectSkillBindingRequest {
    pub project_id: ProjectId,
    pub skill_id: SkillId,
    pub scope: SkillScope,
    pub version: Option<String>,
    pub import_reference: Option<String>,
    pub actor_id: String,
    pub capability: Capability,
    pub policy: ProjectSkillBindingPolicy,
    pub approval_id: Option<String>,
    pub trace_id: TraceId,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectSkillMutationRequest {
    pub project_id: ProjectId,
    pub skill_id: SkillId,
    pub actor_id: String,
    pub capability: Capability,
    pub policy: ProjectSkillBindingPolicy,
    pub approval_id: Option<String>,
    pub trace_id: TraceId,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSkillBindingMutation {
    pub binding: ProjectSkillBinding,
    pub changed: bool,
    pub event_id: Option<EventId>,
}

#[derive(Clone)]
pub struct SqliteProjectSkillBindingRepository {
    pool: Pool<Sqlite>,
}

impl SqliteProjectSkillBindingRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn get(
        &self,
        project_id: &ProjectId,
        skill_id: &SkillId,
    ) -> Result<Option<ProjectSkillBinding>, DomainError> {
        let row = sqlx::query(
            "SELECT project_id, skill_id, scope, current_version, previous_version, import_reference, enabled, actor_id, approval_id, trace_id, revision, created_at, updated_at FROM project_skill_bindings WHERE project_id = ? AND skill_id = ?",
        )
        .bind(project_id.to_string())
        .bind(skill_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| persistence_error("project skill binding query", error))?;
        row.map(decode_binding).transpose()
    }

    pub async fn list(
        &self,
        project_id: &ProjectId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ProjectSkillBinding>, DomainError> {
        let rows = sqlx::query(
            "SELECT project_id, skill_id, scope, current_version, previous_version, import_reference, enabled, actor_id, approval_id, trace_id, revision, created_at, updated_at FROM project_skill_bindings WHERE project_id = ? ORDER BY updated_at DESC, skill_id ASC LIMIT ? OFFSET ?",
        )
        .bind(project_id.to_string())
        .bind(limit.min(MAX_BINDINGS_PER_PROJECT) as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| persistence_error("project skill binding list", error))?;
        rows.into_iter().map(decode_binding).collect()
    }

    pub async fn count_enabled(&self, project_id: &ProjectId) -> Result<usize, DomainError> {
        let row = sqlx::query(
            "SELECT COUNT(*) AS count FROM project_skill_bindings WHERE project_id = ? AND enabled = 1",
        )
        .bind(project_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|error| persistence_error("project skill binding count", error))?;
        let count: i64 = row
            .try_get("count")
            .map_err(|error| persistence_error("project skill binding count decode", error))?;
        usize::try_from(count)
            .map_err(|_| DomainError::InvariantViolation("invalid binding count".into()))
    }

    async fn insert(&self, binding: &ProjectSkillBinding) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO project_skill_bindings (project_id, skill_id, scope, current_version, previous_version, import_reference, enabled, actor_id, approval_id, trace_id, revision, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(binding.project_id.to_string())
        .bind(binding.skill_id.to_string())
        .bind(scope_to_db(binding.scope))
        .bind(&binding.current_version)
        .bind(binding.previous_version.as_deref())
        .bind(binding.import_reference.as_deref())
        .bind(if binding.enabled { 1_i64 } else { 0_i64 })
        .bind(&binding.actor_id)
        .bind(binding.approval_id.as_deref())
        .bind(binding.trace_id.to_string())
        .bind(revision_i64(binding.revision)?)
        .bind(binding.created_at.to_rfc3339())
        .bind(binding.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|error| persistence_error("project skill binding insert", error))?;
        Ok(())
    }

    async fn update(
        &self,
        binding: &ProjectSkillBinding,
        expected_revision: u64,
    ) -> Result<(), DomainError> {
        let result = sqlx::query(
            "UPDATE project_skill_bindings SET scope = ?, current_version = ?, previous_version = ?, import_reference = ?, enabled = ?, actor_id = ?, approval_id = ?, trace_id = ?, revision = ?, updated_at = ? WHERE project_id = ? AND skill_id = ? AND revision = ?",
        )
        .bind(scope_to_db(binding.scope))
        .bind(&binding.current_version)
        .bind(binding.previous_version.as_deref())
        .bind(binding.import_reference.as_deref())
        .bind(if binding.enabled { 1_i64 } else { 0_i64 })
        .bind(&binding.actor_id)
        .bind(binding.approval_id.as_deref())
        .bind(binding.trace_id.to_string())
        .bind(revision_i64(binding.revision)?)
        .bind(binding.updated_at.to_rfc3339())
        .bind(binding.project_id.to_string())
        .bind(binding.skill_id.to_string())
        .bind(revision_i64(expected_revision)?)
        .execute(&self.pool)
        .await
        .map_err(|error| persistence_error("project skill binding update", error))?;
        if result.rows_affected() == 0 {
            return Err(DomainError::ConcurrencyConflict {
                expected: expected_revision.to_string(),
                actual: binding.revision.to_string(),
            });
        }
        Ok(())
    }

    async fn ensure_project_exists(&self, project_id: &ProjectId) -> Result<(), DomainError> {
        let row = sqlx::query("SELECT status FROM projects WHERE id = ?")
            .bind(project_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| persistence_error("project skill project query", error))?;
        let Some(row) = row else {
            return Err(DomainError::NotFound(format!(
                "project not found: {}",
                project_id
            )));
        };
        let status: String = row
            .try_get("status")
            .map_err(|error| persistence_error("project skill status decode", error))?;
        if status == "archived" {
            return Err(DomainError::InvalidStateTransition {
                from: status,
                to: "skill binding mutation".into(),
            });
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct ProjectSkillService {
    skills: SqliteSkillRepository,
    bindings: SqliteProjectSkillBindingRepository,
    loader: crate::skill_loader::SkillLoader,
    event_bus: Option<EventBus<ApplicationEvent>>,
}

impl ProjectSkillService {
    pub fn new(
        skills: SqliteSkillRepository,
        bindings: SqliteProjectSkillBindingRepository,
    ) -> Self {
        Self {
            loader: crate::skill_loader::SkillLoader::new(skills.clone()),
            skills,
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
        request: ProjectSkillBindingRequest,
    ) -> Result<ProjectSkillBindingMutation, DomainError> {
        validate_binding_request(&request)?;
        self.bindings
            .ensure_project_exists(&request.project_id)
            .await?;
        let record = self
            .resolve_record(request.scope, &request.project_id, &request.skill_id)
            .await?;
        validate_record_for_binding(&record, &request)?;
        let version = requested_version(request.version.as_deref(), &record)?;
        let import_reference = normalized_import(request.scope, request.import_reference)?;

        if let Some(existing) = self
            .bindings
            .get(&request.project_id, &request.skill_id)
            .await?
        {
            check_expected_revision(request.expected_revision, existing.revision)?;
            if existing.scope != request.scope {
                return Err(DomainError::InvariantViolation(
                    "existing skill binding scope cannot change implicitly".into(),
                ));
            }
            if existing.scope == request.scope
                && existing.current_version == version
                && existing.import_reference == import_reference
                && existing.enabled
            {
                return Ok(ProjectSkillBindingMutation {
                    binding: existing,
                    changed: false,
                    event_id: None,
                });
            }
            let next_revision = next_revision(existing.revision)?;
            let binding = ProjectSkillBinding {
                project_id: request.project_id,
                skill_id: request.skill_id,
                scope: request.scope,
                current_version: version,
                previous_version: (existing.current_version != record.skill.manifest.version)
                    .then_some(existing.current_version),
                import_reference,
                enabled: true,
                actor_id: request.actor_id,
                approval_id: request.approval_id,
                trace_id: request.trace_id,
                revision: next_revision,
                created_at: existing.created_at,
                updated_at: Utc::now(),
            };
            self.bindings.update(&binding, existing.revision).await?;
            let event_id = self.publish_event("bind", &binding);
            return Ok(ProjectSkillBindingMutation {
                binding,
                changed: true,
                event_id,
            });
        }

        if self.bindings.count_enabled(&request.project_id).await? >= request.policy.max_bindings {
            return Err(DomainError::BudgetExceeded {
                budget_type: "project_skill_bindings".into(),
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
        let binding = ProjectSkillBinding {
            project_id: request.project_id,
            skill_id: request.skill_id,
            scope: request.scope,
            current_version: version,
            previous_version: None,
            import_reference,
            enabled: true,
            actor_id: request.actor_id,
            approval_id: request.approval_id,
            trace_id: request.trace_id,
            revision: 1,
            created_at: now,
            updated_at: now,
        };
        self.bindings.insert(&binding).await?;
        let event_id = self.publish_event("bind", &binding);
        Ok(ProjectSkillBindingMutation {
            binding,
            changed: true,
            event_id,
        })
    }

    pub async fn disable(
        &self,
        request: ProjectSkillMutationRequest,
    ) -> Result<ProjectSkillBindingMutation, DomainError> {
        validate_mutation_request(&request)?;
        self.bindings
            .ensure_project_exists(&request.project_id)
            .await?;
        let existing = self
            .bindings
            .get(&request.project_id, &request.skill_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("project skill binding not found".into()))?;
        check_expected_revision(request.expected_revision, existing.revision)?;
        if !existing.enabled {
            return Ok(ProjectSkillBindingMutation {
                binding: existing,
                changed: false,
                event_id: None,
            });
        }
        let binding = disabled_binding(&existing, &request)?;
        self.bindings.update(&binding, existing.revision).await?;
        let event_id = self.publish_event("disable", &binding);
        Ok(ProjectSkillBindingMutation {
            binding,
            changed: true,
            event_id,
        })
    }

    /// Rollback is deliberately a safe unbind. It removes the active project
    /// reference without rewriting immutable Skill history.
    pub async fn rollback(
        &self,
        request: ProjectSkillMutationRequest,
    ) -> Result<ProjectSkillBindingMutation, DomainError> {
        validate_mutation_request(&request)?;
        self.bindings
            .ensure_project_exists(&request.project_id)
            .await?;
        let existing = self
            .bindings
            .get(&request.project_id, &request.skill_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("project skill binding not found".into()))?;
        check_expected_revision(request.expected_revision, existing.revision)?;
        if !existing.enabled {
            return Ok(ProjectSkillBindingMutation {
                binding: existing,
                changed: false,
                event_id: None,
            });
        }
        let binding = disabled_binding(&existing, &request)?;
        self.bindings.update(&binding, existing.revision).await?;
        let event_id = self.publish_event("rollback", &binding);
        Ok(ProjectSkillBindingMutation {
            binding,
            changed: true,
            event_id,
        })
    }

    pub async fn load_bound(
        &self,
        project_id: ProjectId,
        agent_id: AgentId,
        skill_id: SkillId,
    ) -> Result<crate::skill_loader::LoadedSkill, DomainError> {
        self.load_bound_with_budget(project_id, agent_id, skill_id, SkillLoadBudget::default())
            .await
    }

    pub async fn load_bound_with_budget(
        &self,
        project_id: ProjectId,
        agent_id: AgentId,
        skill_id: SkillId,
        budget: SkillLoadBudget,
    ) -> Result<crate::skill_loader::LoadedSkill, DomainError> {
        self.bindings.ensure_project_exists(&project_id).await?;
        let binding = self
            .bindings
            .get(&project_id, &skill_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("project skill is not bound".into()))?;
        if !binding.enabled {
            return Err(DomainError::NotFound(
                "project skill binding is disabled".into(),
            ));
        }
        let capability =
            Capability::new(Resource::Skill, Action::Read).with_scope(project_id.to_string());
        let trace_id = TraceId::parse(&binding.trace_id.to_string())
            .map_err(|_| DomainError::Validation("binding trace id is invalid".into()))?;
        self.loader
            .load(SkillLoadRequest {
                project_id,
                agent_id,
                skill_id,
                version: Some(binding.current_version),
                scope: binding.scope,
                global_import: binding
                    .import_reference
                    .map(|reference| SkillGlobalImport { reference }),
                capability: capability.clone(),
                policy: SkillLoadPolicy {
                    allow: true,
                    allow_testing: false,
                    allow_external_references: false,
                    allowed_capabilities: CapabilitySet::new().insert(capability),
                },
                budget,
                trace_id,
                requested_paths: Vec::new(),
            })
            .await
            .map_err(load_error)
    }

    async fn resolve_record(
        &self,
        scope: SkillScope,
        project_id: &ProjectId,
        skill_id: &SkillId,
    ) -> Result<SkillRecord, DomainError> {
        let record = match scope {
            SkillScope::Project => self.skills.get(scope, Some(project_id), skill_id).await?,
            SkillScope::Global => self.skills.get(scope, None, skill_id).await?,
        };
        record.ok_or_else(|| DomainError::NotFound("skill source not found".into()))
    }

    fn publish_event(&self, action: &str, binding: &ProjectSkillBinding) -> Option<EventId> {
        let bus = self.event_bus.as_ref()?;
        let event_id = EventId::new();
        let event = ApplicationEvent {
            schema_version: 1,
            event_id,
            event_type: EventKind::SkillBindingChanged,
            project_id: binding.project_id,
            aggregate_id: binding.skill_id.to_string(),
            agent_id: None,
            session_id: None,
            occurred_at: Utc::now(),
            sequence: binding.revision,
            payload: serde_json::json!({
                "action": action,
                "skill_id": binding.skill_id,
                "version": binding.current_version,
                "scope": binding.scope,
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

fn validate_binding_request(request: &ProjectSkillBindingRequest) -> Result<(), DomainError> {
    validate_mutation_context(
        request.project_id,
        &request.actor_id,
        &request.capability,
        &request.policy,
        request.trace_id,
    )?;
    if request.policy.max_bindings == 0 || request.policy.max_bindings > MAX_BINDINGS_PER_PROJECT {
        return Err(DomainError::Validation(
            "invalid project skill binding budget".into(),
        ));
    }
    if let Some(version) = request.version.as_deref() {
        semver::Version::parse(version)
            .map_err(|_| DomainError::Validation("skill version is invalid".into()))?;
    }
    if request.scope == SkillScope::Global {
        if request.version.is_none() {
            return Err(DomainError::Validation(
                "global skill import requires an exact version pin".into(),
            ));
        }
        if request.approval_id.is_none() {
            return Err(DomainError::PermissionDenied {
                capability: "skill.import".into(),
                reason: "global skill import requires explicit approval".into(),
            });
        }
    }
    if request.scope == SkillScope::Project && request.import_reference.is_some() {
        return Err(DomainError::Validation(
            "project skill binding cannot carry a global import reference".into(),
        ));
    }
    validate_optional_text(
        request.approval_id.as_deref(),
        MAX_APPROVAL_ID_BYTES,
        "approval_id",
    )
}

fn validate_mutation_request(request: &ProjectSkillMutationRequest) -> Result<(), DomainError> {
    validate_mutation_context(
        request.project_id,
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
    actor_id: &str,
    capability: &Capability,
    policy: &ProjectSkillBindingPolicy,
    trace_id: TraceId,
) -> Result<(), DomainError> {
    if project_id.as_uuid().is_nil() || trace_id.as_uuid().is_nil() {
        return Err(DomainError::Validation(
            "binding identity is invalid".into(),
        ));
    }
    validate_optional_text(Some(actor_id), MAX_ACTOR_ID_BYTES, "actor_id")?;
    if !policy.allow {
        return Err(DomainError::PermissionDenied {
            capability: capability.to_string(),
            reason: "project skill binding policy denied the operation".into(),
        });
    }
    let required =
        Capability::new(Resource::Skill, Action::Configure).with_scope(project_id.to_string());
    if capability != &required {
        return Err(DomainError::CapabilityUnavailable(
            "skill configure capability is required".into(),
        ));
    }
    if !policy.allowed_capabilities.contains(&required) {
        return Err(DomainError::CapabilityUnavailable(
            "skill configure capability is not allowed by policy".into(),
        ));
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

fn validate_record_for_binding(
    record: &SkillRecord,
    request: &ProjectSkillBindingRequest,
) -> Result<(), DomainError> {
    if record.skill.manifest.scope != request.scope
        || (request.scope == SkillScope::Project
            && record.skill.project_id != Some(request.project_id))
        || (request.scope == SkillScope::Global && record.skill.project_id.is_some())
    {
        return Err(DomainError::PermissionDenied {
            capability: "skill.bind".into(),
            reason: "skill scope does not belong to the requested project".into(),
        });
    }
    if record.skill.status != SkillStatus::Active {
        return Err(DomainError::InvalidStateTransition {
            from: format!("{:?}", record.skill.status),
            to: "bound".into(),
        });
    }
    if request.scope == SkillScope::Global
        && matches!(
            record.skill.manifest.source.kind,
            SkillSourceKind::Git | SkillSourceKind::Registry
        )
    {
        return Err(DomainError::PermissionDenied {
            capability: "skill.import".into(),
            reason: "remote global skill sources are not enabled by this contract".into(),
        });
    }
    for declared in &record.skill.manifest.capabilities {
        if !request.policy.allowed_capabilities.contains(declared) {
            return Err(DomainError::CapabilityUnavailable(
                "skill manifest capability is not allowed by project policy".into(),
            ));
        }
    }
    Ok(())
}

fn requested_version(requested: Option<&str>, record: &SkillRecord) -> Result<String, DomainError> {
    if let Some(requested) = requested {
        if requested != record.skill.manifest.version {
            return Err(DomainError::NotFound(
                "requested skill version is not the current persisted head".into(),
            ));
        }
    }
    Ok(record.skill.manifest.version.clone())
}

fn normalized_import(
    scope: SkillScope,
    reference: Option<String>,
) -> Result<Option<String>, DomainError> {
    match scope {
        SkillScope::Project => Ok(None),
        SkillScope::Global => {
            let reference = reference.ok_or_else(|| DomainError::PermissionDenied {
                capability: "skill.import".into(),
                reason: "global skill requires an explicit project import".into(),
            })?;
            validate_optional_text(
                Some(&reference),
                MAX_IMPORT_REFERENCE_BYTES,
                "import_reference",
            )?;
            if !reference.starts_with("project-import:") {
                return Err(DomainError::Validation(
                    "global skill import reference must identify a project import".into(),
                ));
            }
            Ok(Some(reference))
        }
    }
}

fn disabled_binding(
    existing: &ProjectSkillBinding,
    request: &ProjectSkillMutationRequest,
) -> Result<ProjectSkillBinding, DomainError> {
    Ok(ProjectSkillBinding {
        enabled: false,
        actor_id: request.actor_id.clone(),
        approval_id: request.approval_id.clone(),
        trace_id: request.trace_id,
        revision: next_revision(existing.revision)?,
        updated_at: Utc::now(),
        ..existing.clone()
    })
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

fn next_revision(current: u64) -> Result<u64, DomainError> {
    current
        .checked_add(1)
        .ok_or_else(|| DomainError::Validation("binding revision overflow".into()))
}

fn revision_i64(value: u64) -> Result<i64, DomainError> {
    i64::try_from(value)
        .map_err(|_| DomainError::Validation("binding revision exceeds storage range".into()))
}

fn scope_to_db(scope: SkillScope) -> &'static str {
    match scope {
        SkillScope::Project => "project",
        SkillScope::Global => "global",
    }
}

fn scope_from_db(value: &str) -> Result<SkillScope, DomainError> {
    match value {
        "project" => Ok(SkillScope::Project),
        "global" => Ok(SkillScope::Global),
        _ => Err(DomainError::Validation(
            "invalid skill binding scope".into(),
        )),
    }
}

fn decode_binding(row: sqlx::sqlite::SqliteRow) -> Result<ProjectSkillBinding, DomainError> {
    let project_id = ProjectId::parse(&row_string(&row, "project_id")?)
        .map_err(|_| DomainError::Validation("invalid binding project id".into()))?;
    let skill_id = SkillId::parse(&row_string(&row, "skill_id")?)
        .map_err(|_| DomainError::Validation("invalid binding skill id".into()))?;
    let trace_id = TraceId::parse(&row_string(&row, "trace_id")?)
        .map_err(|_| DomainError::Validation("invalid binding trace id".into()))?;
    let revision = row_i64(&row, "revision")?;
    let revision = u64::try_from(revision)
        .map_err(|_| DomainError::Validation("invalid binding revision".into()))?;
    let created_at = parse_time(&row_string(&row, "created_at")?)?;
    let updated_at = parse_time(&row_string(&row, "updated_at")?)?;
    let enabled: i64 = row
        .try_get("enabled")
        .map_err(|error| persistence_error("binding enabled decode", error))?;
    Ok(ProjectSkillBinding {
        project_id,
        skill_id,
        scope: scope_from_db(&row_string(&row, "scope")?)?,
        current_version: row_string(&row, "current_version")?,
        previous_version: row_optional_string(&row, "previous_version")?,
        import_reference: row_optional_string(&row, "import_reference")?,
        enabled: enabled == 1,
        actor_id: row_string(&row, "actor_id")?,
        approval_id: row_optional_string(&row, "approval_id")?,
        trace_id,
        revision,
        created_at,
        updated_at,
    })
}

fn row_string(row: &sqlx::sqlite::SqliteRow, column: &str) -> Result<String, DomainError> {
    row.try_get(column)
        .map_err(|error| persistence_error("binding text decode", error))
}

fn row_optional_string(
    row: &sqlx::sqlite::SqliteRow,
    column: &str,
) -> Result<Option<String>, DomainError> {
    row.try_get(column)
        .map_err(|error| persistence_error("binding optional text decode", error))
}

fn row_i64(row: &sqlx::sqlite::SqliteRow, column: &str) -> Result<i64, DomainError> {
    row.try_get(column)
        .map_err(|error| persistence_error("binding integer decode", error))
}

fn parse_time(value: &str) -> Result<DateTime<Utc>, DomainError> {
    DateTime::parse_from_rfc3339(value)
        .map(|time| time.with_timezone(&Utc))
        .map_err(|_| DomainError::Validation("invalid binding timestamp".into()))
}

fn persistence_error(context: &str, error: sqlx::Error) -> DomainError {
    DomainError::InvariantViolation(format!("{context}: {error}"))
}

fn load_error(error: crate::skill_loader::SkillLoadError) -> DomainError {
    match error {
        crate::skill_loader::SkillLoadError::NotFound => {
            DomainError::NotFound("bound skill source not found".into())
        }
        crate::skill_loader::SkillLoadError::PolicyDenied
        | crate::skill_loader::SkillLoadError::CapabilityDenied
        | crate::skill_loader::SkillLoadError::GlobalImportRequired => {
            DomainError::PermissionDenied {
                capability: "skill.read".into(),
                reason: "bound skill cannot be loaded under the active policy".into(),
            }
        }
        other => DomainError::InvariantViolation(format!("bound skill load failed: {other}")),
    }
}
