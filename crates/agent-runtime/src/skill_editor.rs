//! Governed draft editing for declarative Skills.
//!
//! The service parses and validates editor input before persistence, creates
//! immutable draft versions, and never moves an active head or executes an
//! artifact.

use agent_core::{
    Action, BudgetLimits, Capability, CapabilitySet, DomainError, ProjectId, Resource, Skill,
    SkillDiagnosticCode, SkillDiagnosticSeverity, SkillFileInput, SkillId, SkillParseRequest,
    SkillParser, SkillScope, TraceId, DEFAULT_MAX_DOCUMENT_BYTES,
};

use crate::skill_repo::{SkillRecord, SqliteSkillRepository};

pub const SKILL_EDIT_CAPABILITY: &str = "skill.edit";
pub const SKILL_DISCARD_CAPABILITY: &str = "skill.discard";
const MAX_ACTOR_ID_BYTES: usize = 128;

#[derive(Debug, Clone)]
pub struct SkillEditorPolicy {
    pub allow: bool,
    pub allowed_capabilities: CapabilitySet,
    pub max_document_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct SkillDraftRequest {
    pub project_id: ProjectId,
    pub skill_id: SkillId,
    pub actor_id: String,
    pub capability: Capability,
    pub policy: SkillEditorPolicy,
    pub budget: BudgetLimits,
    pub trace_id: TraceId,
    pub expected_revision: u64,
    pub base_version: String,
    pub document: String,
    pub files: Vec<SkillFileInput>,
}

#[derive(Debug, Clone)]
pub struct SkillDraftResult {
    pub record: SkillRecord,
    pub changed: bool,
    pub quarantined: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillDraftValidation {
    pub valid: bool,
    pub quarantined: bool,
    pub diagnostics: Vec<SkillValidationDiagnostic>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SkillDiscardRequest {
    pub project_id: ProjectId,
    pub skill_id: SkillId,
    pub version: String,
    pub actor_id: String,
    pub trace_id: TraceId,
    pub expected_revision: u64,
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillValidationDiagnostic {
    pub code: String,
    pub severity: String,
    pub line: usize,
}

#[derive(Clone)]
pub struct SkillDraftService {
    skills: SqliteSkillRepository,
}

impl SkillDraftService {
    pub fn new(skills: SqliteSkillRepository) -> Self {
        Self { skills }
    }

    pub async fn save(&self, request: SkillDraftRequest) -> Result<SkillDraftResult, DomainError> {
        validate_policy(
            request.project_id,
            &request.actor_id,
            &request.capability,
            &request.policy,
            &request.budget,
        )?;
        if request.trace_id.as_uuid().is_nil() {
            return Err(DomainError::Validation(
                "skill editor trace is required".into(),
            ));
        }
        if request.document.len()
            > request
                .policy
                .max_document_bytes
                .min(DEFAULT_MAX_DOCUMENT_BYTES)
        {
            return Err(DomainError::BudgetExceeded {
                budget_type: "skill_editor_document_bytes".into(),
                limit: request.policy.max_document_bytes.to_string(),
                used: request.document.len().to_string(),
            });
        }

        let current = self
            .skills
            .get(
                SkillScope::Project,
                Some(&request.project_id),
                &request.skill_id,
            )
            .await?
            .ok_or_else(|| DomainError::NotFound("skill head not found".into()))?;
        if current.revision != request.expected_revision {
            return Err(DomainError::ConcurrencyConflict {
                expected: request.expected_revision.to_string(),
                actual: current.revision.to_string(),
            });
        }
        if current.skill.manifest.version != request.base_version {
            return Err(DomainError::ConcurrencyConflict {
                expected: request.base_version,
                actual: current.skill.manifest.version,
            });
        }

        let parsed = SkillParser::default()
            .parse(SkillParseRequest {
                document: request.document,
                files: request.files,
                project_id: Some(request.project_id),
            })
            .map_err(|_| DomainError::Validation("skill draft parser validation failed".into()))?;
        if parsed.manifest.id != request.skill_id || parsed.manifest.scope != SkillScope::Project {
            return Err(DomainError::Validation(
                "skill draft identity or scope does not match the project".into(),
            ));
        }
        if parsed.quarantined {
            return Err(DomainError::PermissionDenied {
                capability: SKILL_EDIT_CAPABILITY.into(),
                reason: "skill draft was quarantined by parser diagnostics".into(),
            });
        }
        if parsed.manifest.budget.max_tokens > request.budget.max_tokens
            || parsed.manifest.budget.max_cost_micro_usd > request.budget.max_cost_micro_usd
            || parsed.manifest.budget.max_parallel_invocations
                > request.budget.max_parallel_invocations
            || parsed.manifest.budget.max_wall_time_seconds > request.budget.max_wall_time_seconds
        {
            return Err(DomainError::BudgetExceeded {
                budget_type: "skill_manifest_budget".into(),
                limit: "editor policy".into(),
                used: "requested skill budget exceeds editor budget".into(),
            });
        }

        let draft = Skill::new(parsed.manifest.clone(), Some(request.project_id));
        let (record, changed) = self
            .skills
            .create_draft(&draft, &parsed, request.expected_revision)
            .await?;
        Ok(SkillDraftResult {
            quarantined: record.parsed.quarantined,
            record,
            changed,
        })
    }

    pub async fn validate(
        &self,
        project_id: ProjectId,
        skill_id: SkillId,
        base_version: &str,
        document: String,
        files: Vec<SkillFileInput>,
    ) -> Result<SkillDraftValidation, DomainError> {
        let current = self
            .skills
            .get(SkillScope::Project, Some(&project_id), &skill_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("skill head not found".into()))?;
        if current.skill.manifest.version != base_version {
            return Err(DomainError::ConcurrencyConflict {
                expected: base_version.into(),
                actual: current.skill.manifest.version,
            });
        }

        let parsed = match SkillParser::default().parse(SkillParseRequest {
            document,
            files,
            project_id: Some(project_id),
        }) {
            Ok(parsed) => parsed,
            Err(_) => {
                return Ok(SkillDraftValidation {
                    valid: false,
                    quarantined: false,
                    diagnostics: Vec::new(),
                    errors: vec!["skill draft parser validation failed".into()],
                });
            }
        };
        let mut errors = Vec::new();
        if parsed.manifest.id != skill_id || parsed.manifest.scope != SkillScope::Project {
            errors.push("skill draft identity or scope does not match the project".into());
        }
        let diagnostics = parsed
            .diagnostics
            .iter()
            .map(|diagnostic| SkillValidationDiagnostic {
                code: match diagnostic.code {
                    SkillDiagnosticCode::ExternalLink => "external_link",
                    SkillDiagnosticCode::InstructionOverride => "instruction_override",
                }
                .into(),
                severity: match diagnostic.severity {
                    SkillDiagnosticSeverity::Warning => "warning",
                    SkillDiagnosticSeverity::Quarantine => "quarantine",
                }
                .into(),
                line: diagnostic.line,
            })
            .collect::<Vec<_>>();
        Ok(SkillDraftValidation {
            valid: errors.is_empty() && !parsed.quarantined,
            quarantined: parsed.quarantined,
            diagnostics,
            errors,
        })
    }

    pub async fn discard(&self, request: SkillDiscardRequest) -> Result<SkillRecord, DomainError> {
        let project_id = request.project_id;
        let capability =
            Capability::new(Resource::Skill, Action::Configure).with_scope(project_id.to_string());
        if !request.confirmed
            || request.actor_id.trim().is_empty()
            || request.actor_id.len() > MAX_ACTOR_ID_BYTES
        {
            return Err(DomainError::Validation(
                "skill draft discard confirmation is required".into(),
            ));
        }
        if request.trace_id.as_uuid().is_nil() {
            return Err(DomainError::Validation("skill trace is required".into()));
        }
        let policy = SkillEditorPolicy {
            allow: true,
            allowed_capabilities: CapabilitySet::new().insert(capability),
            max_document_bytes: DEFAULT_MAX_DOCUMENT_BYTES,
        };
        validate_policy(
            project_id,
            &request.actor_id,
            &Capability::new(Resource::Skill, Action::Configure).with_scope(project_id.to_string()),
            &policy,
            &BudgetLimits::default(),
        )?;
        if request.version.trim().is_empty() || request.version.len() > 64 {
            return Err(DomainError::Validation(
                "skill draft version is invalid".into(),
            ));
        }
        self.skills
            .discard_draft(
                SkillScope::Project,
                Some(&project_id),
                &request.skill_id,
                &request.version,
                request.expected_revision,
            )
            .await
    }
}

fn validate_policy(
    project_id: ProjectId,
    actor_id: &str,
    capability: &Capability,
    policy: &SkillEditorPolicy,
    budget: &BudgetLimits,
) -> Result<(), DomainError> {
    if actor_id.trim().is_empty() || actor_id.len() > MAX_ACTOR_ID_BYTES {
        return Err(DomainError::Validation(
            "skill editor actor is invalid".into(),
        ));
    }
    let expected =
        Capability::new(Resource::Skill, Action::Configure).with_scope(project_id.to_string());
    if capability != &expected || !policy.allow || !policy.allowed_capabilities.contains(&expected)
    {
        return Err(DomainError::PermissionDenied {
            capability: capability.to_string(),
            reason: "skill editor capability is not authorized".into(),
        });
    }
    if policy.max_document_bytes == 0 || policy.max_document_bytes > DEFAULT_MAX_DOCUMENT_BYTES {
        return Err(DomainError::BudgetExceeded {
            budget_type: "skill_editor_document_bytes".into(),
            limit: DEFAULT_MAX_DOCUMENT_BYTES.to_string(),
            used: policy.max_document_bytes.to_string(),
        });
    }
    budget.validate()
}
