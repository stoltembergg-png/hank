//! Governed Tauri bridge for the project skill inventory.
//!
//! The webview receives bounded metadata only. It cannot address SQLite,
//! construct the runtime capability, or mutate a binding without an explicit
//! confirmation envelope and an optimistic revision.

use agent_core::{
    error::DomainErrorCode, Action, BudgetLimits, Capability, CapabilitySet, DomainError,
    ProjectId, Resource, SkillCompatibility, SkillFileInput, SkillId, SkillScope, SkillSourceKind,
    SkillStatus, TraceId,
};
use agent_runtime::{
    project_skills::{
        ProjectSkillBinding, ProjectSkillBindingPolicy, ProjectSkillMutationRequest,
        ProjectSkillService, SqliteProjectSkillBindingRepository,
    },
    skill_editor::{SkillDiscardRequest, SkillDraftRequest, SkillDraftService, SkillEditorPolicy},
    skill_repo::{SkillRecord, SqliteSkillRepository},
    sqlite::SqliteStorage,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::State;

pub const SKILL_ROLLBACK_CAPABILITY: &str = "skill.rollback";
pub const SKILL_EDIT_CAPABILITY: &str = "skill.edit";
pub const SKILL_DISCARD_CAPABILITY: &str = "skill.discard";
const MAX_SKILLS_PER_PAGE: usize = 50;
const MAX_OFFSET: usize = 10_000;
const MAX_ACTOR_ID_BYTES: usize = 128;
const MAX_APPROVAL_ID_BYTES: usize = 128;
const MAX_EDITOR_DOCUMENT_BYTES: usize = 64 * 1024;
const MAX_EDITOR_FILES: usize = 32;
const MAX_EDITOR_FILE_BYTES: usize = 16 * 1024;
const MAX_EDITOR_PATH_BYTES: usize = 128;

/// Managed state keeps persistence and the mutation service behind the bridge.
pub struct SkillBridgeState {
    skills: SqliteSkillRepository,
    bindings: SqliteProjectSkillBindingRepository,
    service: ProjectSkillService,
    editor: SkillDraftService,
}

impl SkillBridgeState {
    pub fn new(
        skills: SqliteSkillRepository,
        bindings: SqliteProjectSkillBindingRepository,
    ) -> Self {
        let service = ProjectSkillService::new(skills.clone(), bindings.clone());
        let editor = SkillDraftService::new(skills.clone());
        Self {
            skills,
            bindings,
            service,
            editor,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SkillEditorLoadInput {
    pub project_id: String,
    pub skill_id: String,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SkillEditorFileInput {
    pub path: String,
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SkillEditorPolicyInput {
    pub allow: bool,
    pub max_document_bytes: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SkillEditorBudgetInput {
    pub max_tokens: u64,
    pub max_cost_micro_usd: u64,
    pub max_parallel_invocations: u32,
    pub max_wall_time_seconds: u64,
    pub reset_period: agent_core::ResetPeriod,
}

impl From<SkillEditorBudgetInput> for BudgetLimits {
    fn from(value: SkillEditorBudgetInput) -> Self {
        Self {
            max_tokens: value.max_tokens,
            max_cost_micro_usd: value.max_cost_micro_usd,
            max_parallel_invocations: value.max_parallel_invocations,
            max_wall_time_seconds: value.max_wall_time_seconds,
            reset_period: value.reset_period,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SkillEditorValidateInput {
    pub project_id: String,
    pub skill_id: String,
    pub base_version: String,
    pub document: String,
    pub files: Vec<SkillEditorFileInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SkillEditorSaveInput {
    pub project_id: String,
    pub skill_id: String,
    pub actor_id: String,
    pub trace_id: String,
    pub expected_revision: u64,
    pub base_version: String,
    pub document: String,
    pub files: Vec<SkillEditorFileInput>,
    pub budget: SkillEditorBudgetInput,
    pub policy: SkillEditorPolicyInput,
    pub capability: String,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SkillEditorDiscardInput {
    pub project_id: String,
    pub skill_id: String,
    pub actor_id: String,
    pub trace_id: String,
    pub expected_revision: u64,
    pub version: String,
    pub capability: String,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SkillEditorFileOutput {
    pub path: String,
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SkillEditorDocumentOutput {
    pub project_id: String,
    pub skill_id: String,
    pub base_version: String,
    pub status: SkillStatus,
    pub revision: u64,
    pub manifest_json: String,
    pub markdown: String,
    pub files: Vec<SkillEditorFileOutput>,
    pub policy: SkillPolicySummary,
    pub budget: SkillBudgetSummary,
    pub trace_id: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SkillEditorValidationOutput {
    pub valid: bool,
    pub quarantined: bool,
    pub diagnostics: Vec<SkillEditorDiagnosticOutput>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SkillEditorDiagnosticOutput {
    pub code: String,
    pub severity: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SkillEditorDraftOutput {
    pub project_id: String,
    pub skill_id: String,
    pub version: String,
    pub status: SkillStatus,
    pub content_hash: String,
    pub changed: bool,
    pub quarantined: bool,
    pub revision: u64,
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
            Self::ConfirmationRequired => write!(formatter, "skill mutation confirmation required"),
            Self::MutationRejected { code } => {
                write!(formatter, "skill mutation rejected: {code:?}")
            }
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

    let capability =
        Capability::new(Resource::Skill, Action::Configure).with_scope(project_id.to_string());
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

#[tauri::command]
pub async fn get_skill_editor(
    state: State<'_, SkillBridgeState>,
    input: SkillEditorLoadInput,
) -> Result<SkillEditorDocumentOutput, SkillBridgeError> {
    let project_id = parse_project_id(&input.project_id)?;
    let skill_id = parse_skill_id(&input.skill_id)?;
    if let Some(version) = input.version.as_deref() {
        validate_version_text(version)?;
    }
    let head = state
        .skills
        .get(SkillScope::Project, Some(&project_id), &skill_id)
        .await
        .map_err(SkillBridgeError::from)?
        .ok_or(SkillBridgeError::MutationRejected {
            code: DomainErrorCode::NotFound,
        })?;
    let record = match input.version.as_deref() {
        Some(version) => state
            .skills
            .get_version(SkillScope::Project, Some(&project_id), &skill_id, version)
            .await
            .map_err(SkillBridgeError::from)?
            .ok_or(SkillBridgeError::MutationRejected {
                code: DomainErrorCode::NotFound,
            })?,
        None => head.clone(),
    };
    validate_editor_record(&record, &project_id, &skill_id)?;
    editor_document_output(record, head.revision)
}

#[tauri::command]
pub async fn validate_skill_draft(
    state: State<'_, SkillBridgeState>,
    input: SkillEditorValidateInput,
) -> Result<SkillEditorValidationOutput, SkillBridgeError> {
    let (project_id, skill_id) = validate_editor_document_input(
        &input.project_id,
        &input.skill_id,
        &input.base_version,
        &input.document,
        &input.files,
    )?;
    let report = state
        .editor
        .validate(
            project_id,
            skill_id,
            &input.base_version,
            input.document,
            editor_files(input.files)?,
        )
        .await
        .map_err(SkillBridgeError::from)?;
    Ok(SkillEditorValidationOutput {
        valid: report.valid,
        quarantined: report.quarantined,
        diagnostics: report
            .diagnostics
            .into_iter()
            .map(|diagnostic| SkillEditorDiagnosticOutput {
                code: diagnostic.code,
                severity: diagnostic.severity,
                line: diagnostic.line,
            })
            .collect(),
        errors: report.errors,
    })
}

#[tauri::command]
pub async fn save_skill_draft(
    state: State<'_, SkillBridgeState>,
    input: SkillEditorSaveInput,
) -> Result<SkillEditorDraftOutput, SkillBridgeError> {
    if !input.confirmed {
        return Err(SkillBridgeError::ConfirmationRequired);
    }
    if input.capability != SKILL_EDIT_CAPABILITY {
        return Err(SkillBridgeError::MutationRejected {
            code: DomainErrorCode::PermissionDenied,
        });
    }
    validate_text(&input.actor_id, MAX_ACTOR_ID_BYTES)?;
    let (project_id, skill_id) = validate_editor_document_input(
        &input.project_id,
        &input.skill_id,
        &input.base_version,
        &input.document,
        &input.files,
    )?;
    let trace_id = parse_trace_id(&input.trace_id)?;
    if trace_id.as_uuid().is_nil() {
        return Err(SkillBridgeError::InvalidInput);
    }
    if input.expected_revision == 0
        || !input.policy.allow
        || input.policy.max_document_bytes == 0
        || input.policy.max_document_bytes > MAX_EDITOR_DOCUMENT_BYTES
    {
        return Err(SkillBridgeError::InvalidInput);
    }
    let capability =
        Capability::new(Resource::Skill, Action::Configure).with_scope(project_id.to_string());
    let budget: BudgetLimits = input.budget.into();
    let result = state
        .editor
        .save(SkillDraftRequest {
            project_id,
            skill_id,
            actor_id: input.actor_id,
            capability: capability.clone(),
            policy: SkillEditorPolicy {
                allow: input.policy.allow,
                allowed_capabilities: CapabilitySet::new().insert(capability),
                max_document_bytes: input.policy.max_document_bytes,
            },
            budget,
            trace_id,
            expected_revision: input.expected_revision,
            base_version: input.base_version,
            document: input.document,
            files: editor_files(input.files)?,
        })
        .await
        .map_err(SkillBridgeError::from)?;
    Ok(SkillEditorDraftOutput {
        project_id: result
            .record
            .skill
            .project_id
            .map_or_else(|| project_id.to_string(), |value| value.to_string()),
        skill_id: result.record.skill.manifest.id.to_string(),
        version: result.record.skill.manifest.version,
        status: result.record.skill.status,
        content_hash: result.record.content_hash,
        changed: result.changed,
        quarantined: result.quarantined,
        revision: result.record.revision,
    })
}

#[tauri::command]
pub async fn discard_skill_draft(
    state: State<'_, SkillBridgeState>,
    input: SkillEditorDiscardInput,
) -> Result<SkillEditorDraftOutput, SkillBridgeError> {
    if !input.confirmed {
        return Err(SkillBridgeError::ConfirmationRequired);
    }
    if input.capability != SKILL_DISCARD_CAPABILITY {
        return Err(SkillBridgeError::MutationRejected {
            code: DomainErrorCode::PermissionDenied,
        });
    }
    validate_text(&input.actor_id, MAX_ACTOR_ID_BYTES)?;
    validate_version_text(&input.version)?;
    let project_id = parse_project_id(&input.project_id)?;
    let skill_id = parse_skill_id(&input.skill_id)?;
    let trace_id = parse_trace_id(&input.trace_id)?;
    if input.expected_revision == 0 {
        return Err(SkillBridgeError::InvalidInput);
    }
    let record = state
        .editor
        .discard(SkillDiscardRequest {
            project_id,
            skill_id,
            version: input.version,
            actor_id: input.actor_id,
            trace_id,
            expected_revision: input.expected_revision,
            confirmed: input.confirmed,
        })
        .await
        .map_err(SkillBridgeError::from)?;
    Ok(SkillEditorDraftOutput {
        project_id: project_id.to_string(),
        skill_id: record.skill.manifest.id.to_string(),
        version: record.skill.manifest.version,
        status: record.skill.status,
        content_hash: record.content_hash,
        changed: true,
        quarantined: record.parsed.quarantined,
        revision: record.revision,
    })
}

fn validate_editor_document_input(
    project_id: &str,
    skill_id: &str,
    base_version: &str,
    document: &str,
    files: &[SkillEditorFileInput],
) -> Result<(ProjectId, SkillId), SkillBridgeError> {
    let project_id = parse_project_id(project_id)?;
    let skill_id = parse_skill_id(skill_id)?;
    validate_version_text(base_version)?;
    if document.len() > MAX_EDITOR_DOCUMENT_BYTES || files.len() > MAX_EDITOR_FILES {
        return Err(SkillBridgeError::InvalidInput);
    }
    for file in files {
        validate_text(&file.path, MAX_EDITOR_PATH_BYTES)?;
        validate_text(&file.role, 32)?;
        if file.content.len() > MAX_EDITOR_FILE_BYTES
            || !matches!(
                file.role.as_str(),
                "instruction" | "script" | "template" | "reference" | "test" | "manifest"
            )
        {
            return Err(SkillBridgeError::InvalidInput);
        }
    }
    Ok((project_id, skill_id))
}

fn editor_files(files: Vec<SkillEditorFileInput>) -> Result<Vec<SkillFileInput>, SkillBridgeError> {
    Ok(files
        .into_iter()
        .map(|file| SkillFileInput {
            path: file.path,
            content: file.content,
        })
        .collect())
}

fn validate_version_text(value: &str) -> Result<(), SkillBridgeError> {
    if value.trim().is_empty()
        || value.len() > 64
        || value.chars().any(char::is_control)
        || value.split('.').count() != 3
        || value.split('.').any(|part| {
            part.is_empty() || !part.chars().all(|character| character.is_ascii_digit())
        })
    {
        return Err(SkillBridgeError::InvalidInput);
    }
    Ok(())
}

fn validate_editor_record(
    record: &SkillRecord,
    project_id: &ProjectId,
    skill_id: &SkillId,
) -> Result<(), SkillBridgeError> {
    if record.skill.manifest.id != *skill_id
        || record.skill.manifest.scope != SkillScope::Project
        || record.skill.project_id != Some(*project_id)
        || matches!(
            record.skill.status,
            SkillStatus::Archived | SkillStatus::Blocked
        )
    {
        return Err(SkillBridgeError::MutationRejected {
            code: DomainErrorCode::PermissionDenied,
        });
    }
    Ok(())
}

fn editor_document_output(
    record: SkillRecord,
    revision: u64,
) -> Result<SkillEditorDocumentOutput, SkillBridgeError> {
    let manifest_json = serde_json::to_string_pretty(&record.skill.manifest)
        .map_err(|_| SkillBridgeError::InvalidInput)?;
    let markdown = record
        .parsed
        .instructions
        .iter()
        .map(|section| {
            format!(
                "{} {}\n{}",
                "#".repeat(section.level as usize),
                section.heading,
                section.content
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let files: Vec<SkillEditorFileOutput> = record
        .parsed
        .artifacts
        .into_iter()
        .map(|file| SkillEditorFileOutput {
            path: file.path,
            role: format!("{:?}", file.role).to_ascii_lowercase(),
            content: file.content,
        })
        .collect();
    if manifest_json.len() + markdown.len() > MAX_EDITOR_DOCUMENT_BYTES
        || files.len() > MAX_EDITOR_FILES
        || files
            .iter()
            .any(|file| file.content.len() > MAX_EDITOR_FILE_BYTES)
    {
        return Err(SkillBridgeError::InvalidInput);
    }
    Ok(SkillEditorDocumentOutput {
        project_id: record
            .skill
            .project_id
            .map_or_else(String::new, |value| value.to_string()),
        skill_id: record.skill.manifest.id.to_string(),
        base_version: record.skill.manifest.version.clone(),
        status: record.skill.status,
        revision,
        manifest_json,
        markdown,
        files,
        policy: SkillPolicySummary {
            requires_approval: record.skill.manifest.policy.requires_approval,
            allow_runtime_mutation: record.skill.manifest.policy.allow_runtime_mutation,
            allow_instruction_override: record.skill.manifest.policy.allow_instruction_override,
        },
        budget: SkillBudgetSummary {
            max_tokens: record.skill.manifest.budget.max_tokens,
            max_cost_micro_usd: record.skill.manifest.budget.max_cost_micro_usd,
            max_parallel_invocations: record.skill.manifest.budget.max_parallel_invocations,
            max_wall_time_seconds: record.skill.manifest.budget.max_wall_time_seconds,
            reset_period: record.skill.manifest.budget.reset_period,
        },
        trace_id: record.skill.manifest.trace.trace_id.to_string(),
        content_hash: record.content_hash,
    })
}

fn normalize_list_input(
    input: &ListSkillsInput,
) -> Result<(SkillScope, usize, usize), SkillBridgeError> {
    let scope = input.scope.unwrap_or(SkillScope::Project);
    let limit = input
        .limit
        .unwrap_or(MAX_SKILLS_PER_PAGE)
        .clamp(1, MAX_SKILLS_PER_PAGE);
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
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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

    #[test]
    fn editor_input_rejects_path_escape_and_unbounded_content() {
        let project_id = "proj-00000000-0000-4000-8000-000000000401";
        let skill_id = "skill-00000000-0000-4000-8000-000000000402";
        let base_version = "1.0.0";
        let escaped = vec![SkillEditorFileInput {
            path: "references/../secret.txt".into(),
            role: "reference".into(),
            content: "safe".into(),
        }];
        assert!(validate_editor_document_input(
            project_id,
            skill_id,
            base_version,
            "---\n{}\n---\n# Body",
            &escaped,
        )
        .is_err());

        let oversized = "x".repeat(MAX_EDITOR_DOCUMENT_BYTES + 1);
        assert!(validate_editor_document_input(
            project_id,
            skill_id,
            base_version,
            &oversized,
            &[],
        )
        .is_err());
    }

    #[test]
    fn editor_capability_aliases_are_not_interchangeable() {
        assert_ne!(SKILL_EDIT_CAPABILITY, SKILL_DISCARD_CAPABILITY);
        assert_eq!(SKILL_EDIT_CAPABILITY, "skill.edit");
        assert_eq!(SKILL_DISCARD_CAPABILITY, "skill.discard");
    }
}
