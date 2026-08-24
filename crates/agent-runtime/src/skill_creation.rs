//! Governed creation of project-scoped Skill drafts.
//!
//! Creation is an explicit, non-activating boundary. The service accepts only
//! caller-supplied data, parses and validates it in memory, runs the
//! deterministic non-privileged fixture harness, and persists a draft through
//! the existing immutable Skill repository. It never resolves references,
//! executes scripts, publishes globally, or changes an active head.

use crate::skill_repo::{SkillRecord, SqliteSkillRepository};
use crate::skill_testing::{DeterministicSkillTestRunner, SkillFixture};
use crate::skill_validation::{
    SkillDependencyNode, SkillValidationPolicy, SkillValidationReport, SkillValidationRequest,
    SkillValidationService, SkillValidationStatus,
};
use agent_core::{
    Action, BudgetLimits, Capability, CapabilitySet, DomainError, ProjectId, Resource, Skill,
    SkillFileInput, SkillId, SkillParseRequest, SkillParser, SkillScope, SkillStatus,
    DEFAULT_MAX_DOCUMENT_BYTES,
};
use agent_protocol::ids::TraceId;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::time::Instant;
use tool_core::{
    error_response, success_response, PolicyDecision, SchemaValidationPolicy, Tool,
    ToolEnvironment, ToolError, ToolOutcome, ToolRequest, ToolSchema,
};

pub const SKILL_CREATE_CAPABILITY: &str = "skill:create";
pub const SKILL_CREATE_TOOL_NAME: &str = "skill.create";
pub const SKILL_CREATE_TOOL_VERSION: &str = "1.0.0";

const MAX_ACTOR_ID_BYTES: usize = 128;

#[derive(Debug, Clone)]
pub struct SkillCreationPolicy {
    pub allow: bool,
    pub allowed_capabilities: CapabilitySet,
    pub max_document_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct SkillCreationRequest {
    pub project_id: ProjectId,
    pub actor_id: String,
    pub capability: Capability,
    pub policy: SkillCreationPolicy,
    pub budget: BudgetLimits,
    pub trace_id: TraceId,
    pub document: String,
    pub files: Vec<SkillFileInput>,
    pub fixture: SkillFixture,
    pub dependency_graph: Vec<SkillDependencyNode>,
}

#[derive(Debug, Clone)]
pub struct SkillCreationResult {
    pub record: SkillRecord,
    pub changed: bool,
    pub validation: SkillValidationReport,
}

#[derive(Debug, Clone)]
pub struct SkillDiscardRequest {
    pub project_id: ProjectId,
    pub skill_id: SkillId,
    pub version: String,
    pub actor_id: String,
    pub capability: Capability,
    pub policy: SkillCreationPolicy,
    pub trace_id: TraceId,
    pub expected_revision: u64,
    pub confirmed: bool,
}

#[derive(Clone)]
pub struct SkillCreationService {
    skills: SqliteSkillRepository,
}

impl SkillCreationService {
    pub fn new(skills: SqliteSkillRepository) -> Self {
        Self { skills }
    }

    pub async fn create(
        &self,
        request: SkillCreationRequest,
    ) -> Result<SkillCreationResult, DomainError> {
        validate_creation_request(&request)?;
        if request.document.len() > request.policy.max_document_bytes {
            return Err(DomainError::BudgetExceeded {
                budget_type: "skill_creation_document_bytes".into(),
                limit: request.policy.max_document_bytes.to_string(),
                used: request.document.len().to_string(),
            });
        }

        let parsed = SkillParser::default()
            .parse(SkillParseRequest {
                document: request.document,
                files: request.files,
                project_id: Some(request.project_id),
            })
            .map_err(|_| DomainError::Validation("skill creation parser rejected input".into()))?;
        if parsed.manifest.scope != SkillScope::Project
            || parsed.provenance.project_id != Some(request.project_id)
        {
            return Err(DomainError::PermissionDenied {
                capability: SKILL_CREATE_CAPABILITY.into(),
                reason: "skill creation is project scoped".into(),
            });
        }

        let test_report = DeterministicSkillTestRunner::run(&request.fixture).map_err(|_| {
            DomainError::PermissionDenied {
                capability: SKILL_CREATE_CAPABILITY.into(),
                reason: "skill fixture contains a privileged or invalid step".into(),
            }
        })?;
        let dependency_graph =
            if request.dependency_graph.is_empty() && parsed.manifest.dependencies.is_empty() {
                vec![SkillDependencyNode {
                    skill_id: parsed.manifest.id,
                    dependencies: Vec::new(),
                }]
            } else {
                request.dependency_graph
            };
        let validation_request = SkillValidationRequest {
            project_id: request.project_id,
            skill_id: parsed.manifest.id,
            version: parsed.manifest.version.clone(),
            actor_id: request.actor_id,
            capability: request.capability,
            policy: SkillValidationPolicy {
                allowed_capabilities: request.policy.allowed_capabilities,
            },
            budget: request.budget,
            trace_id: request.trace_id,
            dependency_graph,
        };
        let validation =
            SkillValidationService::validate(&parsed, &validation_request, Some(&test_report));
        if validation.status != SkillValidationStatus::Passed {
            return Err(DomainError::PermissionDenied {
                capability: SKILL_CREATE_CAPABILITY.into(),
                reason: "skill creation validation did not pass".into(),
            });
        }

        let skill = Skill::new(parsed.manifest.clone(), Some(validation_request.project_id));
        if let Some(existing) = self
            .skills
            .get(
                SkillScope::Project,
                Some(&validation_request.project_id),
                &skill.manifest.id,
            )
            .await?
        {
            if same_draft(&existing, &skill, &parsed) {
                return Ok(SkillCreationResult {
                    record: existing,
                    changed: false,
                    validation,
                });
            }
            return Err(DomainError::Duplicate(
                "skill identity already has a different draft or active version".into(),
            ));
        }

        match self.skills.create(&skill, &parsed).await {
            Ok(record) => Ok(SkillCreationResult {
                record,
                changed: true,
                validation,
            }),
            Err(DomainError::Duplicate(_)) => {
                let existing = self
                    .skills
                    .get(
                        SkillScope::Project,
                        Some(&validation_request.project_id),
                        &skill.manifest.id,
                    )
                    .await?
                    .ok_or_else(|| DomainError::Duplicate("skill creation raced".into()))?;
                if same_draft(&existing, &skill, &parsed) {
                    Ok(SkillCreationResult {
                        record: existing,
                        changed: false,
                        validation,
                    })
                } else {
                    Err(DomainError::Duplicate(
                        "skill identity already has a different draft or active version".into(),
                    ))
                }
            }
            Err(error) => Err(error),
        }
    }

    pub async fn discard(&self, request: SkillDiscardRequest) -> Result<SkillRecord, DomainError> {
        validate_discard_request(&request)?;
        let Some(current) = self
            .skills
            .get(
                SkillScope::Project,
                Some(&request.project_id),
                &request.skill_id,
            )
            .await?
        else {
            return Err(DomainError::NotFound("skill draft not found".into()));
        };
        if current.skill.manifest.version == request.version {
            if current.skill.status == SkillStatus::Archived {
                return Ok(current);
            }
            if current.skill.status != SkillStatus::Draft {
                return Err(DomainError::InvalidStateTransition {
                    from: format!("{:?}", current.skill.status),
                    to: "discarded".into(),
                });
            }
            return self
                .skills
                .archive(
                    SkillScope::Project,
                    Some(&request.project_id),
                    &request.skill_id,
                    request.expected_revision,
                )
                .await;
        }

        if let Some(target) = self
            .skills
            .get_version(
                SkillScope::Project,
                Some(&request.project_id),
                &request.skill_id,
                &request.version,
            )
            .await?
        {
            if target.skill.status == SkillStatus::Archived {
                return Ok(target);
            }
        }
        self.skills
            .discard_draft(
                SkillScope::Project,
                Some(&request.project_id),
                &request.skill_id,
                &request.version,
                request.expected_revision,
            )
            .await
    }
}

fn validate_creation_request(request: &SkillCreationRequest) -> Result<(), DomainError> {
    if request.actor_id.trim().is_empty() || request.actor_id.len() > MAX_ACTOR_ID_BYTES {
        return Err(DomainError::Validation(
            "skill creation actor is invalid".into(),
        ));
    }
    let expected =
        Capability::new(Resource::Skill, Action::Create).with_scope(request.project_id.to_string());
    if request.capability != expected
        || !request.policy.allow
        || !request.policy.allowed_capabilities.contains(&expected)
    {
        return Err(DomainError::PermissionDenied {
            capability: SKILL_CREATE_CAPABILITY.into(),
            reason: "skill creation capability is not authorized".into(),
        });
    }
    if request.policy.max_document_bytes == 0
        || request.policy.max_document_bytes > DEFAULT_MAX_DOCUMENT_BYTES
    {
        return Err(DomainError::BudgetExceeded {
            budget_type: "skill_creation_document_bytes".into(),
            limit: DEFAULT_MAX_DOCUMENT_BYTES.to_string(),
            used: request.policy.max_document_bytes.to_string(),
        });
    }
    if request.trace_id.as_uuid().is_nil() {
        return Err(DomainError::Validation(
            "skill creation trace is required".into(),
        ));
    }
    request.budget.validate()
}

fn validate_discard_request(request: &SkillDiscardRequest) -> Result<(), DomainError> {
    if request.actor_id.trim().is_empty() || request.actor_id.len() > MAX_ACTOR_ID_BYTES {
        return Err(DomainError::Validation(
            "skill discard actor is invalid".into(),
        ));
    }
    let expected =
        Capability::new(Resource::Skill, Action::Delete).with_scope(request.project_id.to_string());
    if request.capability != expected
        || !request.policy.allow
        || !request.policy.allowed_capabilities.contains(&expected)
    {
        return Err(DomainError::PermissionDenied {
            capability: "skill:delete".into(),
            reason: "skill discard capability is not authorized".into(),
        });
    }
    if !request.confirmed || request.version.trim().is_empty() || request.version.len() > 64 {
        return Err(DomainError::Validation(
            "skill discard confirmation is required".into(),
        ));
    }
    if request.trace_id.as_uuid().is_nil() {
        return Err(DomainError::Validation(
            "skill discard trace is required".into(),
        ));
    }
    Ok(())
}

fn same_draft(existing: &SkillRecord, skill: &Skill, parsed: &agent_core::ParsedSkill) -> bool {
    existing.skill.status == SkillStatus::Draft
        && existing.skill.manifest.id == skill.manifest.id
        && existing.skill.manifest.version == skill.manifest.version
        && serde_json::to_vec(&existing.skill.manifest).ok()
            == serde_json::to_vec(&skill.manifest).ok()
        && serde_json::to_vec(&existing.parsed).ok() == serde_json::to_vec(parsed).ok()
}

#[derive(Debug, Deserialize)]
struct SkillCreateInput {
    document: String,
    files: Vec<SkillCreateFile>,
    fixture: SkillFixture,
    dependency_graph: Vec<SkillCreateDependency>,
}

#[derive(Debug, Deserialize)]
struct SkillCreateFile {
    path: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct SkillCreateDependency {
    skill_id: SkillId,
    dependencies: Vec<SkillId>,
}

#[derive(Clone)]
pub struct SkillCreateTool {
    service: SkillCreationService,
    policy: SkillCreationPolicy,
}

impl SkillCreateTool {
    pub fn new(service: SkillCreationService, policy: SkillCreationPolicy) -> Self {
        Self { service, policy }
    }
}

fn skill_create_schema() -> &'static ToolSchema {
    static SCHEMA: OnceLock<ToolSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| ToolSchema {
        name: SKILL_CREATE_TOOL_NAME.into(),
        version: SKILL_CREATE_TOOL_VERSION.into(),
        description: Some("Create a project-scoped governed Skill draft".into()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "document": {"type": "string"},
                "files": {"type": "array", "items": {"type": "object", "properties": {"path": {"type": "string"}, "content": {"type": "string"}}, "required": ["path", "content"], "additionalProperties": false}},
                "fixture": {
                    "type": "object",
                    "properties": {
                        "project_id": {"type": "string"},
                        "skill_id": {"type": "string"},
                        "version": {"type": "string"},
                        "trace_id": {"type": "string"},
                        "steps": {"type": "array", "items": {"type": "object", "additionalProperties": true}},
                        "max_steps": {"type": "integer"}
                    },
                    "required": ["project_id", "skill_id", "version", "trace_id", "steps", "max_steps"],
                    "additionalProperties": false
                },
                "dependency_graph": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "skill_id": {"type": "string"},
                            "dependencies": {"type": "array", "items": {"type": "string"}}
                        },
                        "required": ["skill_id", "dependencies"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["document", "files", "fixture", "dependency_graph"],
            "additionalProperties": false
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "project_id": {"type": "string"},
                "skill_id": {"type": "string"},
                "version": {"type": "string"},
                "status": {"type": "string"},
                "revision": {"type": "integer"},
                "changed": {"type": "boolean"},
                "content_hash": {"type": "string"},
                "validation_report_digest": {"type": "string"}
            },
            "required": ["project_id", "skill_id", "version", "status", "revision", "changed", "content_hash", "validation_report_digest"]
        }),
        capabilities: vec![SKILL_CREATE_CAPABILITY.into()],
        destructive: true,
        environment: ToolEnvironment::Host,
        timeout_seconds: 30,
        max_input_bytes: 512 * 1024,
        max_output_bytes: 8 * 1024,
        metadata: BTreeMap::from([(String::from("lifecycle"), String::from("draft_only"))]),
    })
}

#[async_trait]
impl Tool for SkillCreateTool {
    fn schema(&self) -> &'static ToolSchema {
        skill_create_schema()
    }

    fn can_handle(&self, request: &ToolRequest) -> Result<(), ToolError> {
        if request.tool_name != SKILL_CREATE_TOOL_NAME {
            return Err(ToolError::NotFound {
                name: request.tool_name.clone(),
            });
        }
        if request.tool_version != SKILL_CREATE_TOOL_VERSION {
            return Err(ToolError::VersionNotFound {
                name: request.tool_name.clone(),
                version: request.tool_version.clone(),
            });
        }
        if request.context.capability != SKILL_CREATE_CAPABILITY {
            return Err(ToolError::CapabilityMismatch {
                name: SKILL_CREATE_TOOL_NAME.into(),
                capability: request.context.capability.clone(),
            });
        }
        if request.context.policy_decision != PolicyDecision::Allow
            || request.context.agent_id.is_none()
        {
            return Err(ToolError::PermissionDenied {
                decision: request.context.policy_decision,
            });
        }
        Ok(())
    }

    async fn execute(&self, request: ToolRequest) -> Result<tool_core::ToolResponse, ToolError> {
        let started = Instant::now();
        let duration = || started.elapsed().as_millis().min(u64::MAX as u128) as u64;
        if let Err(_error) = request.validate() {
            return Ok(error_response(
                &request,
                ToolOutcome::Failed,
                "skill creation request is invalid",
                duration(),
            ));
        }
        if let Err(error) = self.can_handle(&request) {
            let outcome = match error {
                ToolError::PermissionDenied { .. } => ToolOutcome::PermissionDenied,
                ToolError::CapabilityMismatch { .. } => ToolOutcome::CapabilityMismatch,
                ToolError::NotFound { .. } | ToolError::VersionNotFound { .. } => {
                    ToolOutcome::NotFound
                }
                _ => ToolOutcome::Failed,
            };
            return Ok(error_response(
                &request,
                outcome,
                "skill creation is not authorized",
                duration(),
            ));
        }
        if let Err(_error) = self
            .schema()
            .validate_input(&request.input, SchemaValidationPolicy::strict())
        {
            return Ok(error_response(
                &request,
                ToolOutcome::SchemaValidationError,
                "skill creation input schema is invalid",
                duration(),
            ));
        }
        let input: SkillCreateInput = match serde_json::from_value(request.input.clone()) {
            Ok(input) => input,
            Err(_) => {
                return Ok(error_response(
                    &request,
                    ToolOutcome::SchemaValidationError,
                    "skill creation input could not be decoded",
                    duration(),
                ));
            }
        };
        let Some(agent_id) = request.context.agent_id else {
            return Ok(error_response(
                &request,
                ToolOutcome::PermissionDenied,
                "skill creation agent identity is required",
                duration(),
            ));
        };
        let capability = Capability::new(Resource::Skill, Action::Create)
            .with_scope(request.context.project_id.to_string());
        let creation = self
            .service
            .create(SkillCreationRequest {
                project_id: request.context.project_id,
                actor_id: agent_id.to_string(),
                capability,
                policy: self.policy.clone(),
                budget: request.context.budget_limits.clone(),
                trace_id: request.context.trace_id,
                document: input.document,
                files: input
                    .files
                    .into_iter()
                    .map(|file| SkillFileInput {
                        path: file.path,
                        content: file.content,
                    })
                    .collect(),
                fixture: input.fixture,
                dependency_graph: input
                    .dependency_graph
                    .into_iter()
                    .map(|node| SkillDependencyNode {
                        skill_id: node.skill_id,
                        dependencies: node.dependencies,
                    })
                    .collect(),
            })
            .await;
        match creation {
            Ok(result) => Ok(success_response(
                &request,
                json!({
                    "project_id": request.context.project_id,
                    "skill_id": result.record.skill.manifest.id,
                    "version": result.record.skill.manifest.version,
                    "status": "draft",
                    "revision": result.record.revision,
                    "changed": result.changed,
                    "content_hash": result.record.content_hash,
                    "validation_report_digest": result.validation.report_digest,
                }),
                duration(),
            )),
            Err(error) => {
                let (outcome, message) = match error {
                    DomainError::PermissionDenied { .. } => (
                        ToolOutcome::PermissionDenied,
                        "skill creation validation or policy rejected the draft",
                    ),
                    DomainError::BudgetExceeded { .. } => (
                        ToolOutcome::BudgetExhausted,
                        "skill creation budget was exceeded",
                    ),
                    DomainError::Duplicate(_) => (
                        ToolOutcome::Failed,
                        "skill creation conflicts with an existing version",
                    ),
                    _ => (ToolOutcome::Failed, "skill creation failed closed"),
                };
                Ok(error_response(&request, outcome, message, duration()))
            }
        }
    }
}
