//! Atomic, project-confined filesystem write with bounded rollback.

use crate::PermissionDecision;
use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemWriteResult {
    pub logical_path: String,
    pub trace_id: TraceId,
    pub bytes_written: usize,
    pub operation_key: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FilesystemWriteError {
    #[error("project is not authorized")]
    ProjectUnauthorized,
    #[error("permission decision does not allow write")]
    PermissionDenied,
    #[error("operation key is required")]
    MissingOperationKey,
    #[error("path must be relative")]
    InvalidPath,
    #[error("path is outside authorized roots")]
    OutsideRoot,
    #[error("root is unavailable")]
    RootUnavailable,
    #[error("write payload exceeds limit")]
    PayloadTooLarge,
    #[error("filesystem operation failed")]
    Filesystem,
    #[error("rollback snapshot is unavailable")]
    SnapshotUnavailable,
}

#[derive(Debug, Clone)]
struct Snapshot {
    path: PathBuf,
    previous: Option<Vec<u8>>,
}

#[derive(Debug)]
pub struct FilesystemWriteTool {
    project_id: ProjectId,
    roots: Vec<PathBuf>,
    max_bytes: usize,
    snapshots: Mutex<BTreeMap<String, Snapshot>>,
}

impl FilesystemWriteTool {
    pub fn new(
        project_id: ProjectId,
        roots: Vec<PathBuf>,
        max_bytes: usize,
    ) -> Result<Self, FilesystemWriteError> {
        if roots.is_empty() || max_bytes == 0 {
            return Err(FilesystemWriteError::RootUnavailable);
        }
        let roots = roots
            .into_iter()
            .map(|root| fs::canonicalize(root).map_err(|_| FilesystemWriteError::RootUnavailable))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            project_id,
            roots,
            max_bytes,
            snapshots: Mutex::new(BTreeMap::new()),
        })
    }

    pub fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub fn write(
        &self,
        project_id: ProjectId,
        logical_path: &str,
        content: &[u8],
        permission: PermissionDecision,
        trace_id: TraceId,
        operation_key: &str,
    ) -> Result<FilesystemWriteResult, FilesystemWriteError> {
        if project_id != self.project_id {
            return Err(FilesystemWriteError::ProjectUnauthorized);
        }
        if !permission.is_allowed() {
            return Err(FilesystemWriteError::PermissionDenied);
        }
        if operation_key.trim().is_empty() {
            return Err(FilesystemWriteError::MissingOperationKey);
        }
        if content.len() > self.max_bytes {
            return Err(FilesystemWriteError::PayloadTooLarge);
        }
        let path = self.resolve_target(logical_path)?;
        let mut snapshots = self
            .snapshots
            .lock()
            .map_err(|_| FilesystemWriteError::Filesystem)?;
        if snapshots.contains_key(operation_key) {
            return Ok(FilesystemWriteResult {
                logical_path: logical_path.to_string(),
                trace_id,
                bytes_written: content.len(),
                operation_key: operation_key.to_string(),
            });
        }
        let previous = path
            .exists()
            .then(|| fs::read(&path).map_err(|_| FilesystemWriteError::Filesystem))
            .transpose()?;
        let parent = path.parent().ok_or(FilesystemWriteError::OutsideRoot)?;
        let temp = parent.join(format!(".hank-write-{}-tmp", trace_id));
        fs::write(&temp, content).map_err(|_| FilesystemWriteError::Filesystem)?;
        if fs::rename(&temp, &path).is_err() {
            let _ = fs::remove_file(&temp);
            return Err(FilesystemWriteError::Filesystem);
        }
        snapshots.insert(operation_key.to_string(), Snapshot { path, previous });
        Ok(FilesystemWriteResult {
            logical_path: logical_path.to_string(),
            trace_id,
            bytes_written: content.len(),
            operation_key: operation_key.to_string(),
        })
    }

    pub fn rollback(&self, operation_key: &str) -> Result<(), FilesystemWriteError> {
        let mut snapshots = self
            .snapshots
            .lock()
            .map_err(|_| FilesystemWriteError::Filesystem)?;
        let snapshot = snapshots
            .remove(operation_key)
            .ok_or(FilesystemWriteError::SnapshotUnavailable)?;
        match snapshot.previous {
            Some(bytes) => fs::write(snapshot.path, bytes),
            None => fs::remove_file(snapshot.path),
        }
        .map_err(|_| FilesystemWriteError::Filesystem)
    }

    fn resolve_target(&self, logical_path: &str) -> Result<PathBuf, FilesystemWriteError> {
        let path = Path::new(logical_path);
        if logical_path.trim().is_empty()
            || path.is_absolute()
            || path.components().any(|c| {
                matches!(
                    c,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(FilesystemWriteError::InvalidPath);
        }
        let parent = path.parent().ok_or(FilesystemWriteError::InvalidPath)?;
        let candidates = self
            .roots
            .iter()
            .map(|root| root.join(parent))
            .filter(|p| p.exists())
            .collect::<Vec<_>>();
        for candidate_parent in candidates {
            let canonical_parent = fs::canonicalize(candidate_parent)
                .map_err(|_| FilesystemWriteError::OutsideRoot)?;
            if self
                .roots
                .iter()
                .any(|root| canonical_parent.starts_with(root))
            {
                let target = canonical_parent
                    .join(path.file_name().ok_or(FilesystemWriteError::InvalidPath)?);
                if target.exists() {
                    let canonical =
                        fs::canonicalize(&target).map_err(|_| FilesystemWriteError::OutsideRoot)?;
                    if !self.roots.iter().any(|root| canonical.starts_with(root)) {
                        return Err(FilesystemWriteError::OutsideRoot);
                    }
                }
                return Ok(target);
            }
        }
        Err(FilesystemWriteError::OutsideRoot)
    }
}
