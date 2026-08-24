//! Governed Tauri bridge for the project skill inventory.
//!
//! The webview receives bounded metadata only. It cannot address SQLite,
//! construct the runtime capability, or mutate a binding without an explicit
//! confirmation envelope and an optimistic revision.

use agent_core::{
    error::DomainErrorCode,
    Action, Capability, CapabilitySet, DomainError, ProjectId, Resource, SkillCompatibility,
    SkillId, SkillScope, SkillSourceKind, SkillStatus, TraceId,
};
use agent_runtime::{
    project_skills::{
        ProjectSkillBinding, ProjectSkillBindingPolicy, ProjectSkillMutationRequest,
        ProjectSkillService, SqliteProjectSkillBindingRepository,
    },
    skill_repo::{SkillRecord, SqliteSkillRepository},
    sqlite::SqliteStorage,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::State;

pub const SKILL_ROLLBACK_CAPABILITY: &str = "skill.rollback";
const MAX_SKILLS_PER_PAGE: usize = 50;
const MAX_OFFSET: usize = 10_000;
const MAX_ACTOR_ID_BYTES: usize = 128;
const MAX_APPROVAL_ID_BYTES: usize = 128;

/// Managed state keeps persistence and the mutation service behind the bridge.
pub struct SkillBridgeState {
    skills: SqliteSkillRepository,
    bindings: SqliteProjectSkillBindingRepository,
    service: ProjectSkillService,
}

impl SkillBridgeState {
    pub fn new(
        skills: SqliteSkillRepository,
        bindings: SqliteProjectSkillBindingRepository,
    ) -> Self {
        let service = ProjectSkillService::new(skills.clone(), bindings.clone());
        Self {
            skills,
            bindings,
            service,
        }
    }
}

pub fn bridge_state(storage: &SqliteStorage) -> SkillBridgeState {
    let skills = SqliteSkillRepository::new(storage.pool().clone());
    let bindings = SqliteProjectSkillBindingRepository::new(storage.pool().clone());
    SkillBridgeState::new(skills, bindings)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ListSkillsInput {
    pub project_id: String,
    pub scope: Option<SkillScope>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ListSkillsOutput {
    pub project_id: String,
    pub scope: SkillScope,
    pub skills: Vec<SkillSummary>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
    pub available: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SkillRollbackInput {
    pub project_id: String,
    pub skill_id: String,
    pub actor_id: String,
    pub trace_id: String,
    pub expected_revision: u64,
    pub approval_id: Option<String>,
    pub capability: String,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillBridgeError {
    InvalidInput,
    ConfirmationRequired,
    MutationRejected { code: DomainErrorCode },
}

impl std::fmt::Display for SkillBridgeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => write!(formatter, "invalid skill bridge input"),
            Self::ConfirmationRequired => write!(formatter, "skill rollback confirmation required"),
            Self::MutationRejected { code } => write!(formatter, "skill mutation rejected: {code:?}"),
        }
    }
}

impl std::error::Error for SkillBridgeError {}

impl From<DomainError> for SkillBridgeError {
    fn from(error: DomainError) -> Self {
        Self::MutationRejected { code: error.code() }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SkillSummary {
    pub id: String,
    pub project_id: Option<String>,
    pub name: String,
    pub description: String,
    pub scope: SkillScope,
    pub status: SkillStatus,
    pub version: String,
    pub pinned_version: Option<String>,
    pub rollback_version: Option<String>,
    pub parent_version: Option<String>,
    pub compatibility: SkillCompatibility,
    pub content_hash: String,
    pub source: SkillSourceSummary,
    pub capabilities: Vec<SkillCapabilitySummary>,
    pub policy: SkillPolicySummary,
    pub budget: SkillBudgetSummary,
    pub trace_id: String,
    pub revision: u64,
    pub binding: Option<SkillBindingSummary>,
    pub versions: Vec<SkillVersionSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SkillSourceSummary {
    pub kind: String,
    pub reference_digest: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SkillCapabilitySummary {
    pub resource: String,
    pub action: String,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SkillPolicySummary {
    pub requires_approval: bool,
    pub allow_runtime_mutation: bool,
    pub allow_instruction_override: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SkillBudgetSummary {
    pub max_tokens: u64,
    pub max_cost_micro_usd: u64,
    pub max_parallel_invocations: u32,
    pub max_wall_time_seconds: u64,
    pub reset_period: agent_core::ResetPeriod,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SkillVersionSummary {
    pub version: String,
    pub status: SkillStatus,
    pub compatibility: SkillCompatibility,
    pub content_hash: String,
    pub parent_version: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SkillBindingSummary {
    pub project_id: String,
    pub scope: SkillScope,
    pub current_version: String,
    pub previous_version: Option<String>,
    pub import_reference: Option<String>,
    pub enabled: bool,
    pub approval_id: Option<String>,
    pub trace_id: String,
    pub revision: u64,
}

#[tauri::command]
pub async fn list_skills(
    state: State<'_, SkillBridgeState>,
    input: ListSkillsInput,
) -> Result<ListSkillsOutput, SkillBridgeError> {
    let project_id = parse_project_id(&input.project_id)?;
    let (scope, limit, offset) = normalize_list_input(&input)?;
    let project_scope = match scope {
        SkillScope::Project => Some(&project_id),
        SkillScope::Global => None,
    };
    let records = state
        .skills
        .list(scope, project_scope, limit, offset)
        .await
        .map_err(SkillBridgeError::from)?;

    let mut summaries = Vec::with_capacity(records.len());
    for record in records {
        let binding = state
            .bindings
            .get(&project_id, &record.skill.manifest.id)
            .await
            .map_err(SkillBridgeError::from)?;
        if binding
            .as_ref()
            .is_some_and(|binding| binding.scope != record.skill.manifest.scope)
        {
            return Err(SkillBridgeError::MutationRejected {
                code: DomainErrorCode::InvariantViolation,
            });
        }
        let versions = state
            .skills
            .list_versions(scope, project_scope, &record.skill.manifest.id)
            .await
            .map_err(SkillBridgeError::from)?;
        summaries.push(summary_for_record(record, versions, binding)?);
    }

    Ok(ListSkillsOutput {
        project_id: project_id.to_string(),
        scope,
        total: summaries.len(),
        skills: summaries,
        limit,
        offset,
        available: true,
    })
}

#[tauri::command]
pub async fn rollback_skill(
    state: State<'_, SkillBridgeState>,
    input: SkillRollbackInput,
) -> Result<SkillSummary, SkillBridgeError> {
    validate_rollback_input(&input)?;
    let project_id = parse_project_id(&input.project_id)?;
    let skill_id = parse_skill_id(&input.skill_id)?;
    let trace_id = parse_trace_id(&input.trace_id)?;
    let binding = state
        .bindings
        .get(&project_id, &skill_id)
        .await
        .map_err(SkillBridgeError::from)?
        .ok_or(SkillBridgeError::MutationRejected {
            code: DomainErrorCode::NotFound,
        })?;
    let project_scope = match binding.scope {
        SkillScope::Project => Some(&project_id),
        SkillScope::Global => None,
    };
    let current_record = state
        .skills
        .get(binding.scope, project_scope, &skill_id)
        .await
        .map_err(SkillBridgeError::from)?
        .ok_or(SkillBridgeError::MutationRejected {
            code: DomainErrorCode::NotFound,
        })?;

    if binding.trace_id != trace_id {
        return Err(SkillBridgeError::MutationRejected {
            code: DomainErrorCode::PermissionDenied,
        });
    }
    validate_binding_for_rollback(&binding, &current_record, &input)?;

    let capability = Capability::new(Resource::Skill, Action::Configure)
        .with_scope(project_id.to_string());
    let policy = ProjectSkillBindingPolicy {
        allow: true,
        allowed_capabilities: CapabilitySet::new().insert(capability.clone()),
        max_bindings: 128,
    };
    let mutation = state
        .service
        .rollback(ProjectSkillMutationRequest {
            project_id,
            skill_id,
            actor_id: input.actor_id,
            capability,
            policy,
            approval_id: input.approval_id,
            trace_id,
            expected_revision: Some(input.expected_revision),
        })
        .await
        .map_err(SkillBridgeError::from)?;

    let project_scope = match mutation.binding.scope {
        SkillScope::Project => Some(&mutation.binding.project_id),
        SkillScope::Global => None,
    };
    let versions = state
        .skills
        .list_versions(mutation.binding.scope, project_scope, &skill_id)
        .await
        .map_err(SkillBridgeError::from)?;
    summary_for_record(current_record, versions, Some(mutation.binding))
}

fn normalize_list_input(
    input: &ListSkillsInput,
) -> Result<(SkillScope, usize, usize), SkillBridgeError> {
    let scope = input.scope.unwrap_or(SkillScope::Project);
    let limit = input.limit.unwrap_or(MAX_SKILLS_PER_PAGE).clamp(1, MAX_SKILLS_PER_PAGE);
    let offset = input.offset.unwrap_or(0).min(MAX_OFFSET);
    Ok((scope, limit, offset))
}

fn validate_rollback_input(input: &SkillRollbackInput) -> Result<(), SkillBridgeError> {
    if !input.confirmed {
        return Err(SkillBridgeError::ConfirmationRequired);
    }
    if input.capability != SKILL_ROLLBACK_CAPABILITY {
        return Err(SkillBridgeError::MutationRejected {
            code: DomainErrorCode::PermissionDenied,
        });
    }
    validate_text(&input.actor_id, MAX_ACTOR_ID_BYTES)?;
    if let Some(approval_id) = input.approval_id.as_deref() {
        validate_text(approval_id, MAX_APPROVAL_ID_BYTES)?;
    }
    Ok(())
}

fn validate_text(value: &str, max_bytes: usize) -> Result<(), SkillBridgeError> {
    if value.trim().is_empty()
        || value.len() > max_bytes
        || value.chars().any(char::is_control)
        || value.contains("..")
    {
        return Err(SkillBridgeError::InvalidInput);
    }
    Ok(())
}

fn validate_binding_for_rollback(
    binding: &ProjectSkillBinding,
    record: &SkillRecord,
    input: &SkillRollbackInput,
) -> Result<(), SkillBridgeError> {
    if !binding.enabled || binding.current_version != record.skill.manifest.version {
        return Err(SkillBridgeError::MutationRejected {
            code: DomainErrorCode::InvalidStateTransition,
        });
    }
    if record.skill.manifest.scope != binding.scope
        || (binding.scope == SkillScope::Project
            && record.skill.project_id != Some(binding.project_id))
        || (binding.scope == SkillScope::Global && record.skill.project_id.is_some())
    {
        return Err(SkillBridgeError::MutationRejected {
            code: DomainErrorCode::PermissionDenied,
        });
    }
    match binding.scope {
        SkillScope::Project if binding.import_reference.is_some() => {
            Err(SkillBridgeError::MutationRejected {
                code: DomainErrorCode::InvariantViolation,
            })
        }
        SkillScope::Global
            if !binding
                .import_reference
                .as_deref()
                .is_some_and(|reference| reference.starts_with("project-import:"))
                || binding.approval_id.is_none()
                || input.approval_id.as_deref() != binding.approval_id.as_deref() =>
        {
            Err(SkillBridgeError::MutationRejected {
                code: DomainErrorCode::PermissionDenied,
            })
        }
        _ => Ok(()),
    }
}

fn parse_project_id(value: &str) -> Result<ProjectId, SkillBridgeError> {
    value.parse().map_err(|_| SkillBridgeError::InvalidInput)
}

fn parse_skill_id(value: &str) -> Result<SkillId, SkillBridgeError> {
    value.parse().map_err(|_| SkillBridgeError::InvalidInput)
}

fn parse_trace_id(value: &str) -> Result<TraceId, SkillBridgeError> {
    value.parse().map_err(|_| SkillBridgeError::InvalidInput)
}

fn summary_for_record(
    record: SkillRecord,
    versions: Vec<SkillRecord>,
    binding: Option<ProjectSkillBinding>,
) -> Result<SkillSummary, SkillBridgeError> {
    let skill = record.skill;
    let manifest = skill.manifest;
    let source = SkillSourceSummary {
        kind: source_kind(&manifest.source.kind),
        reference_digest: digest_reference(&manifest.source.reference),
    };
    let capabilities = manifest
        .capabilities
        .iter()
        .take(32)
        .map(|capability| SkillCapabilitySummary {
            resource: capability.resource.to_string(),
            action: capability.action.to_string(),
            scope: capability.scope.clone(),
        })
        .collect();
    let binding = binding.map(binding_summary);
    let version_summaries = versions
        .into_iter()
        .take(50)
        .map(version_summary)
        .collect::<Vec<_>>();

    Ok(SkillSummary {
        id: manifest.id.to_string(),
        project_id: skill.project_id.map(|id| id.to_string()),
        name: manifest.name,
        description: manifest.description,
        scope: manifest.scope,
        status: skill.status,
        version: manifest.version,
        pinned_version: skill.pinned_version,
        rollback_version: skill.rollback_version,
        parent_version: skill.parent_version,
        compatibility: record.compatibility,
        content_hash: record.content_hash,
        source,
        capabilities,
        policy: SkillPolicySummary {
            requires_approval: manifest.policy.requires_approval,
            allow_runtime_mutation: manifest.policy.allow_runtime_mutation,
            allow_instruction_override: manifest.policy.allow_instruction_override,
        },
        budget: SkillBudgetSummary {
            max_tokens: manifest.budget.max_tokens,
            max_cost_micro_usd: manifest.budget.max_cost_micro_usd,
            max_parallel_invocations: manifest.budget.max_parallel_invocations,
            max_wall_time_seconds: manifest.budget.max_wall_time_seconds,
            reset_period: manifest.budget.reset_period,
        },
        trace_id: manifest.trace.trace_id.to_string(),
        revision: record.revision,
        binding,
        versions: version_summaries,
    })
}

fn version_summary(record: SkillRecord) -> SkillVersionSummary {
    SkillVersionSummary {
        version: record.skill.manifest.version,
        status: record.skill.status,
        compatibility: record.compatibility,
        content_hash: record.content_hash,
        parent_version: record.parent_version,
        created_at: record.skill.manifest.created_at.to_rfc3339(),
    }
}

fn binding_summary(binding: ProjectSkillBinding) -> SkillBindingSummary {
    SkillBindingSummary {
        project_id: binding.project_id.to_string(),
        scope: binding.scope,
        current_version: binding.current_version,
        previous_version: binding.previous_version,
        import_reference: binding.import_reference,
        enabled: binding.enabled,
        approval_id: binding.approval_id,
        trace_id: binding.trace_id.to_string(),
        revision: binding.revision,
    }
}

fn source_kind(kind: &SkillSourceKind) -> String {
    match kind {
        SkillSourceKind::BuiltIn => "built_in",
        SkillSourceKind::Local => "local",
        SkillSourceKind::Git => "git",
        SkillSourceKind::Registry => "registry",
        SkillSourceKind::Imported => "imported",
    }
    .into()
}

fn digest_reference(reference: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(reference.as_bytes());
    format!("{:x}", digest.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> SkillRollbackInput {
        SkillRollbackInput {
            project_id: "proj-00000000-0000-4000-8000-000000000401".into(),
            skill_id: "skill-00000000-0000-4000-8000-000000000402".into(),
            actor_id: "operator-1".into(),
            trace_id: "trace-00000000-0000-4000-8000-000000000403".into(),
            expected_revision: 2,
            approval_id: Some("approval-1".into()),
            capability: SKILL_ROLLBACK_CAPABILITY.into(),
            confirmed: true,
        }
    }

    #[test]
    fn bridge_requires_confirmation_and_ui_capability_alias() {
        let mut unconfirmed = input();
        unconfirmed.confirmed = false;
        assert!(matches!(
            validate_rollback_input(&unconfirmed),
            Err(SkillBridgeError::ConfirmationRequired)
        ));

        let mut wrong_capability = input();
        wrong_capability.capability = "skill:configure".into();
        assert!(matches!(
            validate_rollback_input(&wrong_capability),
            Err(SkillBridgeError::MutationRejected {
                code: DomainErrorCode::PermissionDenied
            })
        ));
    }

    #[test]
    fn list_input_is_bounded_before_reaching_sqlite() {
        let input = ListSkillsInput {
            project_id: "proj-00000000-0000-4000-8000-000000000401".into(),
            scope: Some(SkillScope::Global),
            limit: Some(usize::MAX),
            offset: Some(usize::MAX),
        };
        assert_eq!(
            normalize_list_input(&input).expect("bounded list input"),
            (SkillScope::Global, MAX_SKILLS_PER_PAGE, MAX_OFFSET)
        );
    }

    #[test]
    fn source_digest_does_not_expose_the_reference() {
        let reference = "workspace://private-skill-location";
        let digest = digest_reference(reference);
        assert_eq!(digest.len(), 64);
        assert_ne!(digest, reference);
        assert!(!digest.contains("private"));
    }
}
