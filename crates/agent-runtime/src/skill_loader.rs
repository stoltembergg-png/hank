//! Policy-first loader for persisted Skills.
//!
//! Loading is a read-only data operation. It resolves the repository head,
//! validates scope/lifecycle/capability/policy/budget, and returns bounded
//! instructions and artifacts. It never reads the filesystem, follows a
//! network reference, installs dependencies, changes runtime state, or runs
//! a script.

use crate::skill_repo::{SkillRecord, SqliteSkillRepository};
use agent_core::{
    Action, AgentId, Capability, CapabilitySet, DomainError, ProjectId, Resource, Skill,
    SkillArtifact, SkillId, SkillInstructionSection, SkillLink, SkillLinkKind, SkillScope,
    SkillStatus, TraceId,
};
use semver::{Version, VersionReq};
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use thiserror::Error;

pub const MAX_SKILL_LOAD_FILES: usize = 64;
pub const MAX_SKILL_LOAD_BYTES: usize = 512 * 1024;
pub const MAX_SKILL_LOAD_TOKENS: usize = 128 * 1024;
pub const MAX_SKILL_LOAD_DEPTH: usize = 8;
pub const MAX_SKILL_LOAD_DEPENDENCIES: usize = 32;
pub const MAX_SKILL_LOAD_REQUESTED_PATHS: usize = 64;
pub const MAX_SKILL_LOAD_CACHE_ENTRIES: usize = 64;

#[derive(Debug, Clone, Copy)]
pub struct SkillLoadBudget {
    pub max_files: usize,
    pub max_bytes: usize,
    pub max_tokens: usize,
    pub max_dependency_depth: usize,
    pub max_dependencies: usize,
}

impl Default for SkillLoadBudget {
    fn default() -> Self {
        Self {
            max_files: MAX_SKILL_LOAD_FILES,
            max_bytes: MAX_SKILL_LOAD_BYTES,
            max_tokens: MAX_SKILL_LOAD_TOKENS,
            max_dependency_depth: MAX_SKILL_LOAD_DEPTH,
            max_dependencies: MAX_SKILL_LOAD_DEPENDENCIES,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkillLoadPolicy {
    pub allow: bool,
    pub allow_testing: bool,
    pub allow_external_references: bool,
    pub allowed_capabilities: CapabilitySet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillGlobalImport {
    pub reference: String,
}

#[derive(Debug, Clone)]
pub struct SkillLoadRequest {
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub skill_id: SkillId,
    pub version: Option<String>,
    pub scope: SkillScope,
    pub global_import: Option<SkillGlobalImport>,
    pub capability: Capability,
    pub policy: SkillLoadPolicy,
    pub budget: SkillLoadBudget,
    pub trace_id: TraceId,
    /// Empty means all declared data. A non-empty list is an explicit,
    /// bounded allow-list of relative paths.
    pub requested_paths: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SkillLoadError {
    #[error("skill load request is invalid")]
    InvalidRequest,
    #[error("skill load policy denied")]
    PolicyDenied,
    #[error("skill load capability denied")]
    CapabilityDenied,
    #[error("skill lifecycle state cannot be loaded")]
    LifecycleDenied,
    #[error("global Skill requires an explicit import reference")]
    GlobalImportRequired,
    #[error("skill reference is invalid or not allowed")]
    InvalidReference,
    #[error("skill content is invalid")]
    InvalidContent,
    #[error("skill content is quarantined")]
    Quarantined,
    #[error("skill load budget exceeded")]
    BudgetExceeded,
    #[error("skill dependency cycle detected")]
    DependencyCycle,
    #[error("skill dependency depth exceeded")]
    DependencyDepthExceeded,
    #[error("skill dependency could not be resolved")]
    DependencyUnavailable,
    #[error("skill version is not the current loadable version")]
    VersionUnavailable,
    #[error("skill was not found")]
    NotFound,
    #[error("skill repository operation failed")]
    Repository,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SkillLoadCacheKey {
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub skill_id: SkillId,
    pub scope: SkillScope,
    pub version: String,
    pub revision: u64,
    pub global_import: Option<String>,
    pub requested_paths: Vec<String>,
    pub allow_external_references: bool,
}

#[derive(Debug, Clone)]
pub struct LoadedSkillDependency {
    pub skill_id: SkillId,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct LoadedSkill {
    pub skill: Skill,
    pub instructions: Vec<SkillInstructionSection>,
    /// Artifacts, including scripts, are data only. There is no execution
    /// handle or executable path in this result.
    pub artifacts: Vec<SkillArtifact>,
    pub links: Vec<SkillLink>,
    pub dependencies: Vec<LoadedSkillDependency>,
    pub cache_key: SkillLoadCacheKey,
    pub estimated_bytes: usize,
    pub estimated_tokens: usize,
}

#[derive(Clone)]
pub struct SkillLoader {
    repository: SqliteSkillRepository,
    cache: Arc<Mutex<VecDeque<(SkillLoadCacheKey, LoadedSkill)>>>,
}

impl SkillLoader {
    pub fn new(repository: SqliteSkillRepository) -> Self {
        Self {
            repository,
            cache: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub async fn load(&self, request: SkillLoadRequest) -> Result<LoadedSkill, SkillLoadError> {
        let request = normalize_request(request)?;
        let mut ancestry = Vec::new();
        self.load_inner(request, &mut ancestry, 0).await
    }

    /// Removes all cached versions of a Skill for a project. Versioned keys
    /// already prevent stale reads; this method lets update/rollback callers
    /// release old bounded entries immediately.
    pub fn invalidate(&self, project_id: &ProjectId, skill_id: &SkillId) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.retain(|(key, _)| key.project_id != *project_id || key.skill_id != *skill_id);
        }
    }

    fn load_inner<'a>(
        &'a self,
        request: SkillLoadRequest,
        ancestry: &'a mut Vec<SkillId>,
        depth: usize,
    ) -> Pin<Box<dyn Future<Output = Result<LoadedSkill, SkillLoadError>> + Send + 'a>> {
        Box::pin(async move {
            if depth > request.budget.max_dependency_depth {
                return Err(SkillLoadError::DependencyDepthExceeded);
            }
            if ancestry.contains(&request.skill_id) {
                return Err(SkillLoadError::DependencyCycle);
            }
            ancestry.push(request.skill_id);
            let result = self.load_resolved(request, ancestry, depth).await;
            ancestry.pop();
            result
        })
    }

    async fn load_resolved(
        &self,
        request: SkillLoadRequest,
        ancestry: &mut Vec<SkillId>,
        depth: usize,
    ) -> Result<LoadedSkill, SkillLoadError> {
        let record = match (request.scope, request.version.as_deref()) {
            (SkillScope::Project, Some(version)) => self
                .repository
                .get_version(
                    SkillScope::Project,
                    Some(&request.project_id),
                    &request.skill_id,
                    version,
                )
                .await
                .map_err(map_repository_error)?,
            (SkillScope::Global, Some(version)) => self
                .repository
                .get_version(SkillScope::Global, None, &request.skill_id, version)
                .await
                .map_err(map_repository_error)?,
            (SkillScope::Project, None) => self
                .repository
                .get(
                    SkillScope::Project,
                    Some(&request.project_id),
                    &request.skill_id,
                )
                .await
                .map_err(map_repository_error)?,
            (SkillScope::Global, None) => self
                .repository
                .get(SkillScope::Global, None, &request.skill_id)
                .await
                .map_err(map_repository_error)?,
        }
        .ok_or(SkillLoadError::NotFound)?;
        validate_record(&record, &request)?;

        let key = cache_key(&request, &record);
        if let Some(cached) = self.cache_get(&key)? {
            if fits_budget(&cached, &request.budget) {
                return Ok(cached);
            }
        }

        let mut loaded = materialize(&record, &request)?;
        let mut seen_dependencies = BTreeSet::new();
        for dependency in &record.skill.manifest.dependencies {
            if loaded.dependencies.len() >= request.budget.max_dependencies {
                return Err(SkillLoadError::BudgetExceeded);
            }
            let dependency_request = SkillLoadRequest {
                project_id: request.project_id,
                agent_id: request.agent_id,
                skill_id: dependency.skill_id,
                version: None,
                scope: request.scope,
                global_import: request.global_import.clone(),
                capability: request.capability.clone(),
                policy: request.policy.clone(),
                budget: request.budget,
                trace_id: request.trace_id,
                requested_paths: Vec::new(),
            };
            let dependency_result = self
                .load_inner(dependency_request, ancestry, depth + 1)
                .await;
            let dependency_loaded = match dependency_result {
                Ok(value) => value,
                Err(error @ SkillLoadError::DependencyCycle)
                | Err(error @ SkillLoadError::DependencyDepthExceeded) => return Err(error),
                Err(error) => match error {
                    SkillLoadError::NotFound
                    | SkillLoadError::VersionUnavailable
                    | SkillLoadError::DependencyUnavailable
                        if dependency.optional =>
                    {
                        continue
                    }
                    SkillLoadError::NotFound
                    | SkillLoadError::VersionUnavailable
                    | SkillLoadError::DependencyUnavailable => {
                        return Err(SkillLoadError::DependencyUnavailable)
                    }
                    other => return Err(other),
                },
            };
            let requirement = VersionReq::parse(&dependency.version_req)
                .map_err(|_| SkillLoadError::InvalidContent)?;
            if !requirement.matches(
                &Version::parse(&dependency_loaded.skill.manifest.version)
                    .map_err(|_| SkillLoadError::InvalidContent)?,
            ) {
                if dependency.optional {
                    continue;
                }
                return Err(SkillLoadError::DependencyUnavailable);
            }
            let dependency_key = format!(
                "{}@{}",
                dependency_loaded.skill.manifest.id, dependency_loaded.skill.manifest.version
            );
            if seen_dependencies.insert(dependency_key) {
                loaded.dependencies.push(LoadedSkillDependency {
                    skill_id: dependency_loaded.skill.manifest.id,
                    version: dependency_loaded.skill.manifest.version.clone(),
                });
                loaded.dependencies.extend(dependency_loaded.dependencies);
            }
            if loaded.dependencies.len() > request.budget.max_dependencies {
                return Err(SkillLoadError::BudgetExceeded);
            }
        }

        self.cache_insert(key, loaded.clone())?;
        Ok(loaded)
    }

    fn cache_get(&self, key: &SkillLoadCacheKey) -> Result<Option<LoadedSkill>, SkillLoadError> {
        let cache = self.cache.lock().map_err(|_| SkillLoadError::Repository)?;
        Ok(cache
            .iter()
            .find(|(cached_key, _)| cached_key == key)
            .map(|(_, value)| value.clone()))
    }

    fn cache_insert(
        &self,
        key: SkillLoadCacheKey,
        value: LoadedSkill,
    ) -> Result<(), SkillLoadError> {
        let mut cache = self.cache.lock().map_err(|_| SkillLoadError::Repository)?;
        if let Some(position) = cache.iter().position(|(cached_key, _)| cached_key == &key) {
            cache.remove(position);
        }
        if cache.len() >= MAX_SKILL_LOAD_CACHE_ENTRIES {
            cache.pop_front();
        }
        cache.push_back((key, value));
        Ok(())
    }
}

fn normalize_request(mut request: SkillLoadRequest) -> Result<SkillLoadRequest, SkillLoadError> {
    validate_request(&request)?;
    request.requested_paths.sort();
    Ok(request)
}

fn validate_request(request: &SkillLoadRequest) -> Result<(), SkillLoadError> {
    if request.project_id.as_uuid().is_nil()
        || request.agent_id.as_uuid().is_nil()
        || request.trace_id.as_uuid().is_nil()
    {
        return Err(SkillLoadError::InvalidRequest);
    }
    validate_budget(&request.budget)?;
    if request.policy.allowed_capabilities.capabilities.len() > 64 {
        return Err(SkillLoadError::InvalidRequest);
    }
    if request.capability.resource != Resource::Skill
        || request.capability.action != Action::Read
        || request.capability.scope.as_deref() != Some(&request.project_id.to_string())
        || !request
            .policy
            .allowed_capabilities
            .contains(&request.capability)
    {
        return Err(SkillLoadError::CapabilityDenied);
    }
    if !request.policy.allow {
        return Err(SkillLoadError::PolicyDenied);
    }
    if request.scope == SkillScope::Global && request.global_import.is_none() {
        return Err(SkillLoadError::GlobalImportRequired);
    }
    if request.scope == SkillScope::Project && request.global_import.is_some() {
        return Err(SkillLoadError::InvalidRequest);
    }
    if let Some(version) = &request.version {
        if Version::parse(version).is_err() {
            return Err(SkillLoadError::InvalidRequest);
        }
    }
    if let Some(import) = &request.global_import {
        if import.reference.is_empty()
            || import.reference.len() > 256
            || import.reference.chars().any(char::is_control)
            || import.reference.contains("..")
        {
            return Err(SkillLoadError::InvalidReference);
        }
    }
    if request.requested_paths.len() > MAX_SKILL_LOAD_REQUESTED_PATHS {
        return Err(SkillLoadError::InvalidRequest);
    }
    let mut paths = BTreeSet::new();
    for path in &request.requested_paths {
        validate_relative_path(path)?;
        if !paths.insert(path) {
            return Err(SkillLoadError::InvalidReference);
        }
    }
    Ok(())
}

fn validate_budget(budget: &SkillLoadBudget) -> Result<(), SkillLoadError> {
    if budget.max_files == 0
        || budget.max_files > MAX_SKILL_LOAD_FILES
        || budget.max_bytes == 0
        || budget.max_bytes > MAX_SKILL_LOAD_BYTES
        || budget.max_tokens == 0
        || budget.max_tokens > MAX_SKILL_LOAD_TOKENS
        || budget.max_dependency_depth == 0
        || budget.max_dependency_depth > MAX_SKILL_LOAD_DEPTH
        || budget.max_dependencies == 0
        || budget.max_dependencies > MAX_SKILL_LOAD_DEPENDENCIES
    {
        return Err(SkillLoadError::InvalidRequest);
    }
    Ok(())
}

fn validate_relative_path(path: &str) -> Result<(), SkillLoadError> {
    if path.is_empty()
        || path.len() > 256
        || path.starts_with('/')
        || path.contains('\\')
        || path.chars().any(char::is_control)
        || path
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(SkillLoadError::InvalidReference);
    }
    Ok(())
}

fn validate_record(record: &SkillRecord, request: &SkillLoadRequest) -> Result<(), SkillLoadError> {
    if record.skill.manifest.scope != request.scope
        || record.parsed.manifest.scope != request.scope
        || record.skill.manifest.id != request.skill_id
        || record.parsed.manifest.id != request.skill_id
    {
        return Err(SkillLoadError::InvalidContent);
    }
    match request.scope {
        SkillScope::Project if record.skill.project_id != Some(request.project_id) => {
            return Err(SkillLoadError::InvalidContent)
        }
        SkillScope::Global if record.skill.project_id.is_some() => {
            return Err(SkillLoadError::InvalidContent)
        }
        _ => {}
    }
    if record.parsed.quarantined {
        return Err(SkillLoadError::Quarantined);
    }
    match record.skill.status {
        SkillStatus::Active => {
            if record.skill.manifest.policy.requires_approval {
                return Err(SkillLoadError::PolicyDenied);
            }
        }
        SkillStatus::Testing if request.policy.allow_testing => {}
        _ => return Err(SkillLoadError::LifecycleDenied),
    }
    Ok(())
}

fn materialize(
    record: &SkillRecord,
    request: &SkillLoadRequest,
) -> Result<LoadedSkill, SkillLoadError> {
    let declared = record
        .skill
        .manifest
        .files
        .iter()
        .map(|file| (file.path.as_str(), file.role))
        .collect::<HashMap<_, _>>();
    let all_paths = request.requested_paths.is_empty();
    if !all_paths {
        for path in &request.requested_paths {
            if !declared.contains_key(path.as_str()) {
                return Err(SkillLoadError::InvalidReference);
            }
        }
    }

    for artifact in &record.parsed.artifacts {
        validate_relative_path(&artifact.path)?;
        if declared.get(artifact.path.as_str()).copied() != Some(artifact.role) {
            return Err(SkillLoadError::InvalidContent);
        }
        if artifact.content.chars().any(char::is_control) {
            return Err(SkillLoadError::InvalidContent);
        }
    }

    for link in &record.parsed.links {
        match link.kind {
            SkillLinkKind::External if !request.policy.allow_external_references => {
                return Err(SkillLoadError::PolicyDenied)
            }
            SkillLinkKind::Internal => {
                let path = link.target.split('#').next().unwrap_or_default();
                validate_relative_path(path)?;
                if !declared.contains_key(path) {
                    return Err(SkillLoadError::InvalidReference);
                }
            }
            SkillLinkKind::Anchor | SkillLinkKind::External => {}
        }
    }

    let include_instructions = all_paths || request.requested_paths.iter().any(|p| p == "SKILL.md");
    if include_instructions && !declared.contains_key("SKILL.md") {
        return Err(SkillLoadError::InvalidContent);
    }
    let instructions = if include_instructions {
        record.parsed.instructions.clone()
    } else {
        Vec::new()
    };
    let artifacts = record
        .parsed
        .artifacts
        .iter()
        .filter(|artifact| all_paths || request.requested_paths.contains(&artifact.path))
        .cloned()
        .collect::<Vec<_>>();
    let files = instructions.len().min(1).saturating_add(artifacts.len());
    let bytes = instructions
        .iter()
        .map(|section| section.content.len())
        .chain(artifacts.iter().map(|artifact| artifact.content.len()))
        .try_fold(0usize, usize::checked_add)
        .ok_or(SkillLoadError::BudgetExceeded)?;
    let tokens = bytes.saturating_add(3) / 4;
    if files > request.budget.max_files
        || bytes > request.budget.max_bytes
        || tokens > request.budget.max_tokens
    {
        return Err(SkillLoadError::BudgetExceeded);
    }

    Ok(LoadedSkill {
        skill: record.skill.clone(),
        instructions,
        artifacts,
        links: record.parsed.links.clone(),
        dependencies: Vec::new(),
        cache_key: cache_key(request, record),
        estimated_bytes: bytes,
        estimated_tokens: tokens,
    })
}

fn fits_budget(loaded: &LoadedSkill, budget: &SkillLoadBudget) -> bool {
    let files = loaded.instructions.len().min(1) + loaded.artifacts.len();
    files <= budget.max_files
        && loaded.estimated_bytes <= budget.max_bytes
        && loaded.estimated_tokens <= budget.max_tokens
}

fn cache_key(request: &SkillLoadRequest, record: &SkillRecord) -> SkillLoadCacheKey {
    SkillLoadCacheKey {
        project_id: request.project_id,
        agent_id: request.agent_id,
        skill_id: record.skill.manifest.id,
        scope: request.scope,
        version: record.skill.manifest.version.clone(),
        revision: record.revision,
        global_import: request
            .global_import
            .as_ref()
            .map(|import| import.reference.clone()),
        requested_paths: request.requested_paths.clone(),
        allow_external_references: request.policy.allow_external_references,
    }
}

fn map_repository_error(error: DomainError) -> SkillLoadError {
    match error {
        DomainError::NotFound(_) => SkillLoadError::NotFound,
        DomainError::PermissionDenied { .. } => SkillLoadError::PolicyDenied,
        DomainError::Validation(_) | DomainError::InvalidStateTransition { .. } => {
            SkillLoadError::InvalidContent
        }
        DomainError::BudgetExceeded { .. } => SkillLoadError::BudgetExceeded,
        _ => SkillLoadError::Repository,
    }
}
