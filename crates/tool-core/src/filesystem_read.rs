//! Read-only filesystem tool constrained to canonical project roots.

use crate::{PermissionDecision, ToolError};
use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const DEFAULT_MAX_READ_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemReadResult {
    pub logical_path: String,
    pub trace_id: TraceId,
    pub bytes_read: usize,
    pub truncated: bool,
    pub content: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FilesystemReadError {
    #[error("project is not authorized for this filesystem tool")]
    ProjectUnauthorized,
    #[error("permission decision does not allow filesystem read")]
    PermissionDenied,
    #[error("path must be relative")]
    AbsolutePath,
    #[error("path traversal is not allowed")]
    PathTraversal,
    #[error("path is empty")]
    EmptyPath,
    #[error("authorized root is unavailable")]
    RootUnavailable,
    #[error("path is outside authorized roots")]
    OutsideRoot,
    #[error("filesystem entry is unavailable")]
    NotFound,
    #[error("filesystem read failed")]
    ReadFailed,
    #[error("file content is not valid UTF-8")]
    InvalidUtf8,
    #[error("read limit is invalid")]
    InvalidLimit,
}

#[derive(Debug, Clone)]
pub struct FilesystemReadTool {
    project_id: ProjectId,
    roots: Vec<PathBuf>,
    max_bytes: usize,
}

impl FilesystemReadTool {
    pub fn new(
        project_id: ProjectId,
        roots: Vec<PathBuf>,
        max_bytes: usize,
    ) -> Result<Self, FilesystemReadError> {
        if roots.is_empty() || max_bytes == 0 {
            return Err(FilesystemReadError::InvalidLimit);
        }
        let canonical_roots = roots
            .into_iter()
            .map(|root| fs::canonicalize(root).map_err(|_| FilesystemReadError::RootUnavailable))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            project_id,
            roots: canonical_roots,
            max_bytes,
        })
    }

    pub fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub fn read(
        &self,
        project_id: ProjectId,
        logical_path: &str,
        permission: PermissionDecision,
        trace_id: TraceId,
    ) -> Result<FilesystemReadResult, FilesystemReadError> {
        if project_id != self.project_id {
            return Err(FilesystemReadError::ProjectUnauthorized);
        }
        if !permission.is_allowed() {
            return Err(FilesystemReadError::PermissionDenied);
        }
        let relative = validate_relative_path(logical_path)?;
        let candidate = self
            .roots
            .iter()
            .map(|root| root.join(&relative))
            .find(|candidate| candidate.exists())
            .ok_or(FilesystemReadError::NotFound)?;
        let canonical = fs::canonicalize(&candidate).map_err(|_| FilesystemReadError::NotFound)?;
        if !self.roots.iter().any(|root| canonical.starts_with(root)) {
            return Err(FilesystemReadError::OutsideRoot);
        }
        let bytes = fs::read(&canonical).map_err(|_| FilesystemReadError::ReadFailed)?;
        let truncated = bytes.len() > self.max_bytes;
        let bounded = &bytes[..bytes.len().min(self.max_bytes)];
        let content = std::str::from_utf8(bounded)
            .map_err(|_| FilesystemReadError::InvalidUtf8)?
            .to_owned();
        Ok(FilesystemReadResult {
            logical_path: relative.to_string_lossy().into_owned(),
            trace_id,
            bytes_read: bounded.len(),
            truncated,
            content,
        })
    }
}

fn validate_relative_path(value: &str) -> Result<PathBuf, FilesystemReadError> {
    if value.trim().is_empty() {
        return Err(FilesystemReadError::EmptyPath);
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return Err(FilesystemReadError::AbsolutePath);
    }
    for component in path.components() {
        match component {
            Component::ParentDir => return Err(FilesystemReadError::PathTraversal),
            Component::RootDir | Component::Prefix(_) => {
                return Err(FilesystemReadError::AbsolutePath);
            }
            Component::CurDir => {}
            Component::Normal(_) => {}
        }
    }
    Ok(path.to_path_buf())
}

impl From<FilesystemReadError> for ToolError {
    fn from(error: FilesystemReadError) -> Self {
        ToolError::ExecutionFailed(error.to_string())
    }
}
