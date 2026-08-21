//! Bounded, deterministic directory listing constrained to project roots.

use crate::PermissionDecision;
use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryEntryKind {
    File,
    Directory,
    Symlink,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub name: String,
    pub kind: DirectoryEntryKind,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryListResult {
    pub logical_path: String,
    pub trace_id: TraceId,
    pub entries: Vec<DirectoryEntry>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DirectoryFilter {
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub include_hidden: bool,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DirectoryListError {
    #[error("project is not authorized")]
    ProjectUnauthorized,
    #[error("permission decision does not allow directory listing")]
    PermissionDenied,
    #[error("path is invalid")]
    InvalidPath,
    #[error("path is outside authorized roots")]
    OutsideRoot,
    #[error("root is unavailable")]
    RootUnavailable,
    #[error("directory is unavailable")]
    NotFound,
    #[error("filter is invalid")]
    InvalidFilter,
    #[error("directory listing limit is invalid")]
    InvalidLimit,
}

#[derive(Debug, Clone)]
pub struct DirectoryListTool {
    project_id: ProjectId,
    roots: Vec<PathBuf>,
    max_entries: usize,
}

impl DirectoryListTool {
    pub fn new(
        project_id: ProjectId,
        roots: Vec<PathBuf>,
        max_entries: usize,
    ) -> Result<Self, DirectoryListError> {
        if roots.is_empty() || max_entries == 0 {
            return Err(DirectoryListError::InvalidLimit);
        }
        let roots = roots
            .into_iter()
            .map(|root| fs::canonicalize(root).map_err(|_| DirectoryListError::RootUnavailable))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            project_id,
            roots,
            max_entries,
        })
    }

    pub fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub fn list(
        &self,
        project_id: ProjectId,
        logical_path: &str,
        filter: DirectoryFilter,
        permission: PermissionDecision,
        trace_id: TraceId,
    ) -> Result<DirectoryListResult, DirectoryListError> {
        if project_id != self.project_id {
            return Err(DirectoryListError::ProjectUnauthorized);
        }
        if !permission.is_allowed() {
            return Err(DirectoryListError::PermissionDenied);
        }
        validate_filter(&filter)?;
        let relative = validate_relative(logical_path)?;
        let dir = self.resolve_directory(&relative)?;
        let mut entries = Vec::new();
        for entry in fs::read_dir(&dir).map_err(|_| DirectoryListError::NotFound)? {
            let entry = entry.map_err(|_| DirectoryListError::NotFound)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !filter.include_hidden && name.starts_with('.') {
                continue;
            }
            if filter
                .prefix
                .as_ref()
                .is_some_and(|value| !name.starts_with(value))
            {
                continue;
            }
            if filter
                .suffix
                .as_ref()
                .is_some_and(|value| !name.ends_with(value))
            {
                continue;
            }
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|_| DirectoryListError::NotFound)?;
            let kind = if file_type.is_symlink() {
                let target =
                    fs::canonicalize(&path).map_err(|_| DirectoryListError::OutsideRoot)?;
                if !self.roots.iter().any(|root| target.starts_with(root)) {
                    return Err(DirectoryListError::OutsideRoot);
                }
                DirectoryEntryKind::Symlink
            } else if file_type.is_dir() {
                DirectoryEntryKind::Directory
            } else {
                DirectoryEntryKind::File
            };
            let size_bytes = if file_type.is_file() {
                entry
                    .metadata()
                    .map_err(|_| DirectoryListError::NotFound)?
                    .len()
            } else {
                0
            };
            entries.push(DirectoryEntry {
                name,
                kind,
                size_bytes,
            });
        }
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        let truncated = entries.len() > self.max_entries;
        entries.truncate(self.max_entries);
        Ok(DirectoryListResult {
            logical_path: relative.to_string_lossy().into_owned(),
            trace_id,
            entries,
            truncated,
        })
    }

    fn resolve_directory(&self, relative: &Path) -> Result<PathBuf, DirectoryListError> {
        for root in &self.roots {
            let candidate = root.join(relative);
            if !candidate.exists() {
                continue;
            }
            let canonical =
                fs::canonicalize(candidate).map_err(|_| DirectoryListError::OutsideRoot)?;
            if !canonical.is_dir() {
                return Err(DirectoryListError::NotFound);
            }
            if self
                .roots
                .iter()
                .any(|authorized| canonical.starts_with(authorized))
            {
                return Ok(canonical);
            }
        }
        Err(DirectoryListError::OutsideRoot)
    }
}

fn validate_filter(filter: &DirectoryFilter) -> Result<(), DirectoryListError> {
    for value in [filter.prefix.as_deref(), filter.suffix.as_deref()]
        .into_iter()
        .flatten()
    {
        if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
            return Err(DirectoryListError::InvalidFilter);
        }
    }
    Ok(())
}

fn validate_relative(value: &str) -> Result<PathBuf, DirectoryListError> {
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(DirectoryListError::InvalidPath);
    }
    Ok(path.to_path_buf())
}
