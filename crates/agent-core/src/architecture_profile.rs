//! Perfil puro e advisory para análise arquitetural.
//!
//! Este módulo valida manifests tipados, boundaries, documentos e evidence sem
//! editar arquitetura, executar source/comandos, ratificar ADRs, acessar Git,
//! filesystem, rede, providers ou secrets.

use crate::task_mapping::{MappingState, TaskWorkspaceMapping};
use crate::{DomainError, ProjectId, TaskId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use thiserror::Error;

pub const ARCHITECTURE_PROFILE_SCHEMA_VERSION: u32 = 1;
const MAX_POLICY_REVISION_LEN: usize = 128;
const MAX_GRAPH_REVISION_LEN: usize = 128;
const MAX_ID_LEN: usize = 128;
const MAX_TEXT_LEN: usize = 512;
const MAX_PATH_LEN: usize = 512;
const MAX_LAYERS: usize = 64;
const MAX_EDGES: usize = 256;
const MAX_DOCUMENTS: usize = 64;
const MAX_EVIDENCE: usize = 32;
const MAX_FINDINGS: usize = 128;
const MAX_EVIDENCE_BYTES: u64 = 1_048_576;
const REQUIRED_CHECKS: [ArchitectureCheck; 4] = [
    ArchitectureCheck::Graph,
    ArchitectureCheck::Dependencies,
    ArchitectureCheck::Documents,
    ArchitectureCheck::AdrImpact,
];

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ArchitectureProfileError {
    #[error("invalid architecture profile: {0}")]
    InvalidProfile(String),
    #[error("architecture mapping is not active")]
    MappingInactive,
    #[error("architecture request scope does not match the mapping")]
    ScopeMismatch,
    #[error("architecture tool is not allowlisted or attempts mutation")]
    ToolDenied,
    #[error("architecture path is outside the bounded read scope")]
    PathDenied,
    #[error("invalid architecture request: {0}")]
    InvalidRequest(String),
    #[error("invalid architecture manifest: {0}")]
    InvalidManifest(String),
    #[error("invalid architecture evidence: {0}")]
    InvalidEvidence(String),
}

impl From<ArchitectureProfileError> for DomainError {
    fn from(error: ArchitectureProfileError) -> Self {
        DomainError::Validation(error.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchitectureTool {
    ReadGraph,
    ReadSourceReference,
    ReadAdr,
    ReadDocumentation,
    ReadDiff,
    WriteArchitecture,
    RatifyAdr,
    BypassGate,
}

impl ArchitectureTool {
    fn is_mutating(self) -> bool {
        matches!(
            self,
            Self::WriteArchitecture | Self::RatifyAdr | Self::BypassGate
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureAgentProfile {
    pub schema_version: u32,
    pub policy_revision: String,
    pub allowed_tools: Vec<ArchitectureTool>,
    pub max_layers: usize,
    pub max_edges: usize,
    pub max_documents: usize,
    pub max_evidence: usize,
    pub max_findings: usize,
}

impl Default for ArchitectureAgentProfile {
    fn default() -> Self {
        Self {
            schema_version: ARCHITECTURE_PROFILE_SCHEMA_VERSION,
            policy_revision: "architecture-v1".into(),
            allowed_tools: vec![
                ArchitectureTool::ReadGraph,
                ArchitectureTool::ReadSourceReference,
                ArchitectureTool::ReadAdr,
                ArchitectureTool::ReadDocumentation,
                ArchitectureTool::ReadDiff,
            ],
            max_layers: MAX_LAYERS,
            max_edges: MAX_EDGES,
            max_documents: MAX_DOCUMENTS,
            max_evidence: MAX_EVIDENCE,
            max_findings: MAX_FINDINGS,
        }
    }
}

impl ArchitectureAgentProfile {
    pub fn validate(&self) -> Result<(), ArchitectureProfileError> {
        if self.schema_version != ARCHITECTURE_PROFILE_SCHEMA_VERSION
            || !bounded_text(&self.policy_revision, MAX_POLICY_REVISION_LEN)
            || contains_instruction_like(&self.policy_revision)
        {
            return Err(ArchitectureProfileError::InvalidProfile(
                "schema or policy revision is invalid".into(),
            ));
        }
        if self.allowed_tools.is_empty() || self.allowed_tools.len() > 16 {
            return Err(ArchitectureProfileError::InvalidProfile(
                "tool allowlist is outside bounds".into(),
            ));
        }
        let mut tools = BTreeSet::new();
        if self
            .allowed_tools
            .iter()
            .any(|tool| tool.is_mutating() || !tools.insert(*tool))
        {
            return Err(ArchitectureProfileError::InvalidProfile(
                "allowlist contains mutation or duplicate tool".into(),
            ));
        }
        if self.max_layers == 0
            || self.max_layers > MAX_LAYERS
            || self.max_edges == 0
            || self.max_edges > MAX_EDGES
            || self.max_documents == 0
            || self.max_documents > MAX_DOCUMENTS
            || self.max_evidence == 0
            || self.max_evidence > MAX_EVIDENCE
            || self.max_findings == 0
            || self.max_findings > MAX_FINDINGS
        {
            return Err(ArchitectureProfileError::InvalidProfile(
                "architecture limits are outside bounds".into(),
            ));
        }
        Ok(())
    }

    pub fn authorize(
        &self,
        mapping: &TaskWorkspaceMapping,
        request: &ArchitectureRequest,
    ) -> Result<ArchitecturePermit, ArchitectureProfileError> {
        self.validate()?;
        if mapping.state() != MappingState::Active {
            return Err(ArchitectureProfileError::MappingInactive);
        }
        request.validate()?;
        if request.project_id != mapping.project_id()
            || request.task_id != mapping.task_id()
            || request.repository_id != mapping.repository_id()
            || request.worktree_id != mapping.worktree_id()
            || request.branch != mapping.branch()
        {
            return Err(ArchitectureProfileError::ScopeMismatch);
        }
        if request.tool.is_mutating() || !self.allowed_tools.contains(&request.tool) {
            return Err(ArchitectureProfileError::ToolDenied);
        }
        Ok(ArchitecturePermit {
            project_id: request.project_id,
            task_id: request.task_id,
            worktree_id: request.worktree_id.clone(),
            branch: request.branch.clone(),
            head_sha: request.head_sha.clone(),
            tree_sha: request.tree_sha.clone(),
            tool: request.tool,
        })
    }

    pub fn evaluate(
        &self,
        mapping: &TaskWorkspaceMapping,
        manifest: &ArchitectureManifest,
    ) -> Result<ArchitectureReport, ArchitectureProfileError> {
        self.validate()?;
        manifest.validate_shape(self)?;
        if manifest.project_id != mapping.project_id()
            || manifest.task_id != mapping.task_id()
            || manifest.repository_id != mapping.repository_id()
            || manifest.worktree_id != mapping.worktree_id()
            || manifest.branch != mapping.branch()
        {
            return Err(ArchitectureProfileError::ScopeMismatch);
        }

        let mut findings = Vec::new();
        let layer_ids: HashSet<&str> = manifest
            .layers
            .iter()
            .map(|layer| layer.id.as_str())
            .collect();
        let forbidden: HashSet<(&str, &str)> = manifest
            .forbidden_edges
            .iter()
            .map(|edge| (edge.from.as_str(), edge.to.as_str()))
            .collect();
        let allowed: HashSet<(&str, &str)> = manifest
            .allowed_edges
            .iter()
            .map(|edge| (edge.from.as_str(), edge.to.as_str()))
            .collect();

        for edge in &manifest.edges {
            if !layer_ids.contains(edge.from.as_str()) || !layer_ids.contains(edge.to.as_str()) {
                findings.push(ArchitectureFinding::new(
                    "UNKNOWN_LAYER",
                    ArchitectureSeverity::High,
                    format!("{} -> {} references an unknown layer", edge.from, edge.to),
                    Some(edge.to.clone()),
                    ArchitectureFindingStatus::Failed,
                )?);
            } else if forbidden.contains(&(edge.from.as_str(), edge.to.as_str())) {
                findings.push(ArchitectureFinding::new(
                    "FORBIDDEN_EDGE",
                    ArchitectureSeverity::Critical,
                    format!(
                        "{} -> {} is forbidden by the architecture boundary",
                        edge.from, edge.to
                    ),
                    Some(edge.to.clone()),
                    ArchitectureFindingStatus::Failed,
                )?);
            } else if !allowed.contains(&(edge.from.as_str(), edge.to.as_str())) {
                findings.push(ArchitectureFinding::new(
                    "UNDECLARED_EDGE",
                    ArchitectureSeverity::High,
                    format!(
                        "{} -> {} is not in the allowed edge set",
                        edge.from, edge.to
                    ),
                    Some(edge.to.clone()),
                    ArchitectureFindingStatus::Failed,
                )?);
            }
        }

        if has_cycle(&manifest.edges) {
            findings.push(ArchitectureFinding::new(
                "CYCLE",
                ArchitectureSeverity::Critical,
                "architecture dependency cycle detected".into(),
                None,
                ArchitectureFindingStatus::Failed,
            )?);
        }

        for document in &manifest.documents {
            if !document.present {
                findings.push(ArchitectureFinding::new(
                    "MISSING_DOCUMENT",
                    ArchitectureSeverity::High,
                    format!("required {} is absent", document.kind.label()),
                    Some(document.path.clone()),
                    ArchitectureFindingStatus::Blocked,
                )?);
            }
        }

        let evidence_by_check: HashMap<ArchitectureCheck, &ArchitectureEvidence> = manifest
            .evidence
            .iter()
            .map(|evidence| (evidence.check, evidence))
            .collect();
        let mut no_proof = false;
        for check in REQUIRED_CHECKS {
            let Some(evidence) = evidence_by_check.get(&check) else {
                no_proof = true;
                findings.push(ArchitectureFinding::new(
                    "MISSING_EVIDENCE",
                    ArchitectureSeverity::High,
                    format!("{} evidence is missing", check.label()),
                    None,
                    ArchitectureFindingStatus::Unknown,
                )?);
                continue;
            };
            if evidence.graph_revision != manifest.graph_revision
                || evidence.head_sha != manifest.head_sha
                || evidence.tree_sha != manifest.tree_sha
                || evidence.policy_revision != manifest.policy_revision
            {
                no_proof = true;
                findings.push(ArchitectureFinding::new(
                    "STALE_EVIDENCE",
                    ArchitectureSeverity::High,
                    format!("{} evidence identity is stale", check.label()),
                    Some(evidence.file_path.clone()),
                    ArchitectureFindingStatus::Unknown,
                )?);
            } else if matches!(evidence.status, ArchitectureEvidenceStatus::Passed) {
                continue;
            } else if matches!(evidence.status, ArchitectureEvidenceStatus::Failed) {
                findings.push(ArchitectureFinding::new(
                    "EVIDENCE_FAILED",
                    ArchitectureSeverity::High,
                    format!("{} evidence failed", check.label()),
                    Some(evidence.file_path.clone()),
                    ArchitectureFindingStatus::Failed,
                )?);
            } else {
                no_proof = true;
                findings.push(ArchitectureFinding::new(
                    "INCOMPLETE_EVIDENCE",
                    ArchitectureSeverity::High,
                    format!("{} evidence is not passed", check.label()),
                    Some(evidence.file_path.clone()),
                    ArchitectureFindingStatus::Unknown,
                )?);
            }
        }

        let graph_failure = findings
            .iter()
            .any(|finding| finding.status == ArchitectureFindingStatus::Failed);
        let document_blocker = findings
            .iter()
            .any(|finding| finding.status == ArchitectureFindingStatus::Blocked);
        let status = if graph_failure {
            ArchitectureReportStatus::Failed
        } else if document_blocker {
            ArchitectureReportStatus::Blocked
        } else if no_proof {
            ArchitectureReportStatus::NoProof
        } else {
            ArchitectureReportStatus::Pass
        };
        if findings.len() > self.max_findings {
            return Err(ArchitectureProfileError::InvalidManifest(
                "finding count exceeds profile limit".into(),
            ));
        }
        Ok(ArchitectureReport {
            schema_version: ARCHITECTURE_PROFILE_SCHEMA_VERSION,
            policy_revision: manifest.policy_revision.clone(),
            graph_revision: manifest.graph_revision.clone(),
            project_id: manifest.project_id,
            task_id: manifest.task_id,
            repository_id: manifest.repository_id.clone(),
            worktree_id: manifest.worktree_id.clone(),
            branch: manifest.branch.clone(),
            head_sha: manifest.head_sha.clone(),
            tree_sha: manifest.tree_sha.clone(),
            status,
            findings,
            evidence: manifest.evidence.clone(),
            evaluator_digest: stable_digest(manifest),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureRequest {
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub repository_id: String,
    pub worktree_id: String,
    pub branch: String,
    pub head_sha: String,
    pub tree_sha: String,
    pub tool: ArchitectureTool,
    pub path: Option<String>,
}

impl ArchitectureRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: ProjectId,
        task_id: TaskId,
        repository_id: impl Into<String>,
        worktree_id: impl Into<String>,
        branch: impl Into<String>,
        head_sha: impl Into<String>,
        tree_sha: impl Into<String>,
        tool: ArchitectureTool,
        path: Option<String>,
    ) -> Result<Self, ArchitectureProfileError> {
        let request = Self {
            project_id,
            task_id,
            repository_id: repository_id.into(),
            worktree_id: worktree_id.into(),
            branch: branch.into(),
            head_sha: head_sha.into(),
            tree_sha: tree_sha.into(),
            tool,
            path,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn with_tool(mut self, tool: ArchitectureTool) -> Self {
        self.tool = tool;
        self
    }

    pub fn head_sha(&self) -> &str {
        &self.head_sha
    }

    pub fn tree_sha(&self) -> &str {
        &self.tree_sha
    }

    fn validate(&self) -> Result<(), ArchitectureProfileError> {
        if !bounded_text(&self.repository_id, MAX_ID_LEN)
            || !bounded_text(&self.worktree_id, MAX_ID_LEN)
            || !bounded_text(&self.branch, MAX_PATH_LEN)
            || !valid_sha(&self.head_sha, 40)
            || !valid_sha(&self.tree_sha, 64)
        {
            return Err(ArchitectureProfileError::InvalidRequest(
                "identity or SHA is invalid".into(),
            ));
        }
        if let Some(path) = &self.path {
            if !safe_relative_path(path) {
                return Err(ArchitectureProfileError::PathDenied);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchitecturePermit {
    project_id: ProjectId,
    task_id: TaskId,
    worktree_id: String,
    branch: String,
    head_sha: String,
    tree_sha: String,
    tool: ArchitectureTool,
}

impl ArchitecturePermit {
    pub fn can_write(&self) -> bool {
        false
    }

    pub fn can_edit_architecture(&self) -> bool {
        false
    }

    pub fn can_ratify_adr(&self) -> bool {
        false
    }

    pub fn can_bypass_gate(&self) -> bool {
        false
    }

    pub fn head_sha(&self) -> &str {
        &self.head_sha
    }

    pub fn tree_sha(&self) -> &str {
        &self.tree_sha
    }

    pub fn tool(&self) -> ArchitectureTool {
        self.tool
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureLayer {
    pub id: String,
    pub owner: String,
    pub responsibility: String,
    pub allowed_dependencies: Vec<String>,
    pub process_lifecycle: String,
    pub contract: String,
}

impl ArchitectureLayer {
    pub fn new(
        id: impl Into<String>,
        owner: impl Into<String>,
        responsibility: impl Into<String>,
        allowed_dependencies: Vec<String>,
        process_lifecycle: impl Into<String>,
        contract: impl Into<String>,
    ) -> Result<Self, ArchitectureProfileError> {
        let layer = Self {
            id: id.into(),
            owner: owner.into(),
            responsibility: responsibility.into(),
            allowed_dependencies,
            process_lifecycle: process_lifecycle.into(),
            contract: contract.into(),
        };
        validate_layer_shape(&layer)?;
        Ok(layer)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
}

impl ArchitectureEdge {
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
        kind: impl Into<String>,
    ) -> Result<Self, ArchitectureProfileError> {
        let edge = Self {
            from: from.into(),
            to: to.into(),
            kind: kind.into(),
        };
        if !bounded_text(&edge.from, MAX_ID_LEN)
            || !bounded_text(&edge.to, MAX_ID_LEN)
            || !bounded_text(&edge.kind, MAX_ID_LEN)
            || contains_instruction_like(&edge.kind)
        {
            return Err(ArchitectureProfileError::InvalidManifest(
                "edge identity or kind is invalid".into(),
            ));
        }
        Ok(edge)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureEdgeRef {
    pub from: String,
    pub to: String,
}

impl ArchitectureEdgeRef {
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Result<Self, ArchitectureProfileError> {
        let edge = Self {
            from: from.into(),
            to: to.into(),
        };
        if !bounded_text(&edge.from, MAX_ID_LEN) || !bounded_text(&edge.to, MAX_ID_LEN) {
            return Err(ArchitectureProfileError::InvalidManifest(
                "edge reference is invalid".into(),
            ));
        }
        Ok(edge)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchitectureDocumentKind {
    Architecture,
    Adr,
}

impl ArchitectureDocumentKind {
    fn label(self) -> &'static str {
        match self {
            Self::Architecture => "architecture document",
            Self::Adr => "ADR",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureDocument {
    pub path: String,
    pub kind: ArchitectureDocumentKind,
    pub present: bool,
    pub digest: String,
}

impl ArchitectureDocument {
    pub fn new(
        path: impl Into<String>,
        kind: ArchitectureDocumentKind,
        present: bool,
        digest: impl Into<String>,
    ) -> Result<Self, ArchitectureProfileError> {
        let document = Self {
            path: path.into(),
            kind,
            present,
            digest: digest.into(),
        };
        if !safe_relative_path(&document.path)
            || !valid_document_digest(&document.digest, document.present)
        {
            return Err(ArchitectureProfileError::InvalidManifest(
                "document path or digest is invalid".into(),
            ));
        }
        Ok(document)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchitectureCheck {
    Graph,
    Dependencies,
    Documents,
    AdrImpact,
}

impl ArchitectureCheck {
    fn label(self) -> &'static str {
        match self {
            Self::Graph => "graph",
            Self::Dependencies => "dependencies",
            Self::Documents => "documents",
            Self::AdrImpact => "ADR impact",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchitectureEvidenceStatus {
    Passed,
    Failed,
    Missing,
    Skipped,
    NoRun,
    Malformed,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureEvidence {
    pub check: ArchitectureCheck,
    pub file_path: String,
    pub head_sha: String,
    pub tree_sha: String,
    pub graph_revision: String,
    pub policy_revision: String,
    pub digest: String,
    pub bytes: u64,
    pub status: ArchitectureEvidenceStatus,
}

impl ArchitectureEvidence {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        check: ArchitectureCheck,
        file_path: impl Into<String>,
        head_sha: impl Into<String>,
        tree_sha: impl Into<String>,
        graph_revision: impl Into<String>,
        policy_revision: impl Into<String>,
        digest: impl Into<String>,
        bytes: u64,
        status: ArchitectureEvidenceStatus,
    ) -> Result<Self, ArchitectureProfileError> {
        let evidence = Self {
            check,
            file_path: file_path.into(),
            head_sha: head_sha.into(),
            tree_sha: tree_sha.into(),
            graph_revision: graph_revision.into(),
            policy_revision: policy_revision.into(),
            digest: digest.into(),
            bytes,
            status,
        };
        if !safe_relative_path(&evidence.file_path)
            || !valid_sha(&evidence.head_sha, 40)
            || !valid_sha(&evidence.tree_sha, 64)
            || !bounded_text(&evidence.graph_revision, MAX_GRAPH_REVISION_LEN)
            || !bounded_text(&evidence.policy_revision, MAX_POLICY_REVISION_LEN)
            || !valid_evidence_payload(&evidence)
        {
            return Err(ArchitectureProfileError::InvalidEvidence(
                "evidence identity, digest or byte bound is invalid".into(),
            ));
        }
        Ok(evidence)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureManifest {
    pub schema_version: u32,
    pub policy_revision: String,
    pub graph_revision: String,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub repository_id: String,
    pub worktree_id: String,
    pub branch: String,
    pub head_sha: String,
    pub tree_sha: String,
    pub layers: Vec<ArchitectureLayer>,
    pub edges: Vec<ArchitectureEdge>,
    pub allowed_edges: Vec<ArchitectureEdgeRef>,
    pub forbidden_edges: Vec<ArchitectureEdgeRef>,
    pub documents: Vec<ArchitectureDocument>,
    pub evidence: Vec<ArchitectureEvidence>,
}

impl ArchitectureManifest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request: &ArchitectureRequest,
        graph_revision: impl Into<String>,
        layers: Vec<ArchitectureLayer>,
        edges: Vec<ArchitectureEdge>,
        allowed_edges: Vec<ArchitectureEdgeRef>,
        forbidden_edges: Vec<ArchitectureEdgeRef>,
        documents: Vec<ArchitectureDocument>,
        evidence: Vec<ArchitectureEvidence>,
    ) -> Result<Self, ArchitectureProfileError> {
        let manifest = Self {
            schema_version: ARCHITECTURE_PROFILE_SCHEMA_VERSION,
            policy_revision: "architecture-v1".into(),
            graph_revision: graph_revision.into(),
            project_id: request.project_id,
            task_id: request.task_id,
            repository_id: request.repository_id.clone(),
            worktree_id: request.worktree_id.clone(),
            branch: request.branch.clone(),
            head_sha: request.head_sha.clone(),
            tree_sha: request.tree_sha.clone(),
            layers,
            edges,
            allowed_edges,
            forbidden_edges,
            documents,
            evidence,
        };
        manifest.validate_shape(&ArchitectureAgentProfile::default())?;
        Ok(manifest)
    }

    fn validate_shape(
        &self,
        profile: &ArchitectureAgentProfile,
    ) -> Result<(), ArchitectureProfileError> {
        if self.schema_version != ARCHITECTURE_PROFILE_SCHEMA_VERSION
            || self.policy_revision != profile.policy_revision
            || !bounded_text(&self.graph_revision, MAX_GRAPH_REVISION_LEN)
            || contains_instruction_like(&self.graph_revision)
            || !bounded_text(&self.repository_id, MAX_ID_LEN)
            || !bounded_text(&self.worktree_id, MAX_ID_LEN)
            || !bounded_text(&self.branch, MAX_PATH_LEN)
            || !valid_sha(&self.head_sha, 40)
            || !valid_sha(&self.tree_sha, 64)
        {
            return Err(ArchitectureProfileError::InvalidManifest(
                "manifest schema, identity or revision is invalid".into(),
            ));
        }
        if self.layers.is_empty()
            || self.layers.len() > profile.max_layers
            || self.edges.len() > profile.max_edges
            || self.allowed_edges.len() > profile.max_edges
            || self.forbidden_edges.len() > profile.max_edges
            || self.documents.len() > profile.max_documents
            || self.evidence.len() > profile.max_evidence
        {
            return Err(ArchitectureProfileError::InvalidManifest(
                "manifest collection exceeds bounded limits".into(),
            ));
        }
        let layer_ids: HashSet<&str> = self.layers.iter().map(|layer| layer.id.as_str()).collect();
        if layer_ids.len() != self.layers.len() {
            return Err(ArchitectureProfileError::InvalidManifest(
                "duplicate layer id".into(),
            ));
        }
        let mut owners = HashSet::new();
        for layer in &self.layers {
            validate_layer_shape(layer)?;
            if !owners.insert(layer.owner.as_str()) {
                return Err(ArchitectureProfileError::InvalidManifest(
                    "duplicate layer owner".into(),
                ));
            }
            for dependency in &layer.allowed_dependencies {
                if dependency != "ports/application contracts"
                    && !layer_ids.contains(dependency.as_str())
                {
                    return Err(ArchitectureProfileError::InvalidManifest(
                        "layer contains unknown dependency".into(),
                    ));
                }
            }
        }
        validate_unique_edges(&self.edges, |edge| (&edge.from, &edge.to))?;
        validate_unique_edges(&self.allowed_edges, |edge| (&edge.from, &edge.to))?;
        validate_unique_edges(&self.forbidden_edges, |edge| (&edge.from, &edge.to))?;
        for edge in self
            .edges
            .iter()
            .map(|edge| (&edge.from, &edge.to))
            .chain(self.allowed_edges.iter().map(|edge| (&edge.from, &edge.to)))
            .chain(
                self.forbidden_edges
                    .iter()
                    .map(|edge| (&edge.from, &edge.to)),
            )
        {
            if !layer_ids.contains(edge.0.as_str()) || !layer_ids.contains(edge.1.as_str()) {
                return Err(ArchitectureProfileError::InvalidManifest(
                    "edge references unknown layer".into(),
                ));
            }
        }
        let mut documents = HashSet::new();
        for document in &self.documents {
            if !documents.insert(document.path.as_str())
                || !safe_relative_path(&document.path)
                || !valid_document_digest(&document.digest, document.present)
            {
                return Err(ArchitectureProfileError::InvalidManifest(
                    "document is duplicate or malformed".into(),
                ));
            }
        }
        let mut checks = HashSet::new();
        for evidence in &self.evidence {
            if !checks.insert(evidence.check)
                || !safe_relative_path(&evidence.file_path)
                || !valid_sha(&evidence.head_sha, 40)
                || !valid_sha(&evidence.tree_sha, 64)
                || !valid_evidence_payload(evidence)
            {
                return Err(ArchitectureProfileError::InvalidManifest(
                    "evidence is duplicate or malformed".into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchitectureReportStatus {
    Pass,
    Failed,
    Blocked,
    NoProof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchitectureSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchitectureFindingStatus {
    Failed,
    Blocked,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureFinding {
    code: String,
    severity: ArchitectureSeverity,
    summary: String,
    reference: Option<String>,
    status: ArchitectureFindingStatus,
}

impl ArchitectureFinding {
    fn new(
        code: impl Into<String>,
        severity: ArchitectureSeverity,
        summary: String,
        reference: Option<String>,
        status: ArchitectureFindingStatus,
    ) -> Result<Self, ArchitectureProfileError> {
        let finding = Self {
            code: code.into(),
            severity,
            summary,
            reference,
            status,
        };
        if !bounded_text(&finding.code, MAX_ID_LEN)
            || !bounded_text(&finding.summary, MAX_TEXT_LEN)
            || finding
                .reference
                .as_ref()
                .is_some_and(|reference| !safe_relative_path(reference))
            || contains_instruction_like(&finding.summary)
        {
            return Err(ArchitectureProfileError::InvalidManifest(
                "finding is malformed or instruction-like".into(),
            ));
        }
        Ok(finding)
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn severity(&self) -> ArchitectureSeverity {
        self.severity
    }

    pub fn status(&self) -> ArchitectureFindingStatus {
        self.status
    }

    pub fn reference(&self) -> Option<&str> {
        self.reference.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureReport {
    schema_version: u32,
    policy_revision: String,
    graph_revision: String,
    project_id: ProjectId,
    task_id: TaskId,
    repository_id: String,
    worktree_id: String,
    branch: String,
    head_sha: String,
    tree_sha: String,
    status: ArchitectureReportStatus,
    findings: Vec<ArchitectureFinding>,
    evidence: Vec<ArchitectureEvidence>,
    evaluator_digest: String,
}

impl ArchitectureReport {
    pub fn status(&self) -> ArchitectureReportStatus {
        self.status
    }

    pub fn findings(&self) -> &[ArchitectureFinding] {
        &self.findings
    }

    pub fn validate(
        &self,
        profile: &ArchitectureAgentProfile,
        mapping: &TaskWorkspaceMapping,
    ) -> Result<(), ArchitectureProfileError> {
        profile.validate()?;
        if self.schema_version != ARCHITECTURE_PROFILE_SCHEMA_VERSION
            || self.policy_revision != profile.policy_revision
            || self.project_id != mapping.project_id()
            || self.task_id != mapping.task_id()
            || self.repository_id != mapping.repository_id()
            || self.worktree_id != mapping.worktree_id()
            || self.branch != mapping.branch()
            || !valid_sha(&self.head_sha, 40)
            || !valid_sha(&self.tree_sha, 64)
            || !valid_sha(&self.evaluator_digest, 16)
        {
            return Err(ArchitectureProfileError::InvalidManifest(
                "report identity or evaluator digest is invalid".into(),
            ));
        }
        if self.status != ArchitectureReportStatus::Pass {
            return Err(ArchitectureProfileError::InvalidManifest(
                "architecture report is not proven pass".into(),
            ));
        }
        if !self.findings.is_empty() {
            return Err(ArchitectureProfileError::InvalidManifest(
                "pass report contains findings".into(),
            ));
        }
        if self.evidence.len() != REQUIRED_CHECKS.len()
            || self.evidence.iter().any(|evidence| {
                evidence.status != ArchitectureEvidenceStatus::Passed
                    || evidence.graph_revision != self.graph_revision
                    || evidence.head_sha != self.head_sha
                    || evidence.tree_sha != self.tree_sha
                    || evidence.policy_revision != self.policy_revision
            })
        {
            return Err(ArchitectureProfileError::InvalidEvidence(
                "pass report evidence is incomplete or stale".into(),
            ));
        }
        Ok(())
    }

    pub fn handoff(&self) -> Option<ArchitectureHandoff> {
        if self.status == ArchitectureReportStatus::Pass {
            return None;
        }
        Some(ArchitectureHandoff {
            status: self.status,
            finding_codes: self
                .findings
                .iter()
                .map(|finding| finding.code.clone())
                .collect(),
            digest: self.evaluator_digest.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureHandoff {
    status: ArchitectureReportStatus,
    finding_codes: Vec<String>,
    digest: String,
}

impl ArchitectureHandoff {
    pub fn is_advisory(&self) -> bool {
        true
    }

    pub fn can_edit_architecture(&self) -> bool {
        false
    }

    pub fn can_ratify_adr(&self) -> bool {
        false
    }

    pub fn can_bypass_gate(&self) -> bool {
        false
    }

    pub fn can_approve(&self) -> bool {
        false
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn status(&self) -> ArchitectureReportStatus {
        self.status
    }

    pub fn finding_codes(&self) -> &[String] {
        &self.finding_codes
    }
}

fn validate_layer_shape(layer: &ArchitectureLayer) -> Result<(), ArchitectureProfileError> {
    if !bounded_text(&layer.id, MAX_ID_LEN)
        || !bounded_text(&layer.owner, MAX_ID_LEN)
        || !bounded_text(&layer.responsibility, MAX_TEXT_LEN)
        || layer.allowed_dependencies.len() > MAX_LAYERS
        || !bounded_text(&layer.process_lifecycle, MAX_TEXT_LEN)
        || !bounded_text(&layer.contract, MAX_TEXT_LEN)
        || contains_instruction_like(&layer.id)
        || contains_instruction_like(&layer.owner)
        || contains_instruction_like(&layer.responsibility)
        || contains_instruction_like(&layer.process_lifecycle)
        || contains_instruction_like(&layer.contract)
    {
        return Err(ArchitectureProfileError::InvalidManifest(
            "layer is malformed or instruction-like".into(),
        ));
    }
    Ok(())
}

fn validate_unique_edges<T, F>(edges: &[T], key: F) -> Result<(), ArchitectureProfileError>
where
    F: Fn(&T) -> (&String, &String),
{
    let mut seen = HashSet::new();
    if edges.iter().any(|edge| {
        let (from, to) = key(edge);
        !seen.insert((from.as_str(), to.as_str()))
    }) {
        return Err(ArchitectureProfileError::InvalidManifest(
            "duplicate architecture edge".into(),
        ));
    }
    Ok(())
}

fn has_cycle(edges: &[ArchitectureEdge]) -> bool {
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in edges {
        adjacency.entry(&edge.from).or_default().push(&edge.to);
    }
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    fn visit<'a>(
        node: &'a str,
        adjacency: &HashMap<&'a str, Vec<&'a str>>,
        visiting: &mut HashSet<&'a str>,
        visited: &mut HashSet<&'a str>,
    ) -> bool {
        if visiting.contains(node) {
            return true;
        }
        if visited.contains(node) {
            return false;
        }
        visiting.insert(node);
        if adjacency.get(node).is_some_and(|next| {
            next.iter()
                .any(|child| visit(child, adjacency, visiting, visited))
        }) {
            return true;
        }
        visiting.remove(node);
        visited.insert(node);
        false
    }
    adjacency
        .keys()
        .copied()
        .any(|node| visit(node, &adjacency, &mut visiting, &mut visited))
}

fn bounded_text(value: &str, max: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= max
        && !value.chars().any(char::is_control)
        && !contains_instruction_like(value)
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PATH_LEN
        && !value.starts_with('/')
        && !value.starts_with('\\')
        && !value.split(['/', '\\']).any(|part| part == "..")
        && !value.chars().any(char::is_control)
}

fn valid_sha(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_document_digest(digest: &str, present: bool) -> bool {
    if digest.chars().any(char::is_control) || digest.len() > 64 {
        return false;
    }
    if present {
        valid_sha(digest, 64)
    } else {
        digest.is_empty() || valid_sha(digest, 64)
    }
}

fn valid_evidence_payload(evidence: &ArchitectureEvidence) -> bool {
    if evidence.bytes > MAX_EVIDENCE_BYTES
        || evidence.digest.chars().any(char::is_control)
        || evidence.digest.len() > 64
    {
        return false;
    }
    match evidence.status {
        ArchitectureEvidenceStatus::Passed
        | ArchitectureEvidenceStatus::Failed
        | ArchitectureEvidenceStatus::Stale => {
            evidence.bytes > 0 && valid_sha(&evidence.digest, 64)
        }
        ArchitectureEvidenceStatus::Missing
        | ArchitectureEvidenceStatus::Skipped
        | ArchitectureEvidenceStatus::NoRun => {
            evidence.digest.is_empty() || valid_sha(&evidence.digest, 64)
        }
        ArchitectureEvidenceStatus::Malformed => !evidence.digest.is_empty(),
    }
}

fn contains_instruction_like(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    [
        "ignore previous",
        "system prompt",
        "execute command",
        "run shell",
        "sudo ",
        "rm -rf",
        "bypass gate",
        "approve this",
    ]
    .iter()
    .any(|marker| value.contains(marker))
}

fn stable_digest(manifest: &ArchitectureManifest) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    let input = format!(
        "{}:{}:{}:{}:{}:{}",
        manifest.policy_revision,
        manifest.graph_revision,
        manifest.head_sha,
        manifest.tree_sha,
        manifest.layers.len(),
        manifest.edges.len()
    );
    for byte in input.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
