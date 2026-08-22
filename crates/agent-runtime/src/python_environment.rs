//! Project-scoped, reproducible Python environment manifests.
//!
//! This slice records and rolls back validated environment intent. It does not
//! install packages or mutate a global Python installation.

use agent_protocol::ids::ProjectId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

const MAX_NAME: usize = 128;
const MAX_PACKAGES: usize = 128;
const MAX_SOURCES: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PythonPackageRequirement {
    pub name: String,
    pub version: String,
    pub sha256: String,
}

impl PythonPackageRequirement {
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        sha256: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            sha256: sha256.into(),
        }
    }

    fn validate(&self) -> Result<(), PythonEnvironmentError> {
        if !bounded_name(&self.name)
            || !bounded_name(&self.version)
            || self.sha256.len() != 64
            || !self.sha256.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err(PythonEnvironmentError::InvalidPackage);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PythonEnvironmentManifest {
    pub schema_version: u32,
    pub project_id: ProjectId,
    pub environment_id: String,
    pub python_version: String,
    pub packages: Vec<PythonPackageRequirement>,
    pub source_allowlist: Vec<String>,
}

impl PythonEnvironmentManifest {
    pub fn new(
        project_id: ProjectId,
        environment_id: impl Into<String>,
        python_version: impl Into<String>,
        mut packages: Vec<PythonPackageRequirement>,
        source_allowlist: Vec<String>,
    ) -> Result<Self, PythonEnvironmentError> {
        if packages.len() > MAX_PACKAGES
            || source_allowlist.is_empty()
            || source_allowlist.len() > MAX_SOURCES
        {
            return Err(PythonEnvironmentError::InvalidManifest);
        }
        packages.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
        let mut identities = BTreeSet::new();
        for package in &packages {
            package.validate()?;
            if !identities.insert(format!("{}@{}", package.name, package.version)) {
                return Err(PythonEnvironmentError::DuplicatePackage);
            }
        }
        let environment_id = environment_id.into();
        if !bounded_name(&environment_id) {
            return Err(PythonEnvironmentError::InvalidManifest);
        }
        let python_version = python_version.into();
        if !bounded_name(&python_version)
            || source_allowlist.iter().any(|source| !valid_source(source))
        {
            return Err(PythonEnvironmentError::InvalidManifest);
        }
        Ok(Self {
            schema_version: 1,
            project_id,
            environment_id,
            python_version,
            packages,
            source_allowlist,
        })
    }
}

#[derive(Debug, Error)]
pub enum PythonEnvironmentError {
    #[error("environment manifest is invalid")]
    InvalidManifest,
    #[error("package requirement is invalid")]
    InvalidPackage,
    #[error("package requirement is duplicated")]
    DuplicatePackage,
    #[error("environment lock is held")]
    Locked,
    #[error("environment manifest is missing")]
    Missing,
    #[error("environment storage failed: {0}")]
    Storage(#[from] std::io::Error),
    #[error("environment manifest encoding failed: {0}")]
    Encoding(#[from] serde_json::Error),
}

pub struct PythonEnvironmentManager {
    root: PathBuf,
}

impl PythonEnvironmentManager {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn prepare(
        &self,
        manifest: &PythonEnvironmentManifest,
    ) -> Result<(), PythonEnvironmentError> {
        let directory = self.directory(manifest);
        fs::create_dir_all(&directory)?;
        let lock = directory.join("environment.lock");
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock)
        {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(PythonEnvironmentError::Locked)
            }
            Err(error) => return Err(error.into()),
        }
        let result = self.write_atomic(&directory.join("environment.json"), manifest);
        let _ = fs::remove_file(lock);
        result
    }

    pub fn load(
        &self,
        project_id: ProjectId,
        environment_id: &str,
    ) -> Result<PythonEnvironmentManifest, PythonEnvironmentError> {
        let path = self
            .root
            .join(project_id.to_string())
            .join(environment_id)
            .join("environment.json");
        if !path.is_file() {
            return Err(PythonEnvironmentError::Missing);
        }
        Ok(serde_json::from_slice(&fs::read(path)?)?)
    }

    pub fn rollback(
        &self,
        project_id: ProjectId,
        environment_id: &str,
    ) -> Result<(), PythonEnvironmentError> {
        let directory = self.root.join(project_id.to_string()).join(environment_id);
        let previous = directory.join("environment.previous.json");
        if !previous.is_file() {
            return Err(PythonEnvironmentError::Missing);
        }
        fs::rename(previous, directory.join("environment.json"))?;
        Ok(())
    }

    fn directory(&self, manifest: &PythonEnvironmentManifest) -> PathBuf {
        self.root
            .join(manifest.project_id.to_string())
            .join(&manifest.environment_id)
    }

    fn write_atomic(
        &self,
        path: &Path,
        manifest: &PythonEnvironmentManifest,
    ) -> Result<(), PythonEnvironmentError> {
        if path.is_file() {
            fs::rename(path, path.with_file_name("environment.previous.json"))?;
        }
        let temp = path.with_file_name("environment.json.tmp");
        fs::write(&temp, serde_json::to_vec_pretty(manifest)?)?;
        fs::rename(temp, path)?;
        Ok(())
    }
}

fn bounded_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_NAME
        && !value.chars().any(char::is_control)
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains("..")
}

fn valid_source(value: &str) -> bool {
    value.starts_with("https://") && value.len() <= 512 && !value.chars().any(char::is_control)
}
