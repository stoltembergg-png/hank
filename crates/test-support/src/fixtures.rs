//! Deterministic, offline fixtures for contract and integration tests.
//!
//! This module is dev-only support: it creates synthetic bounded data, records a
//! manifest hash, and owns cleanup. It never reads production state or secrets.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};

const MAX_FIXTURE_ID_BYTES: usize = 128;
const MAX_PAYLOAD_BYTES: usize = 64 * 1024;
const MAX_SERIALIZED_FIXTURE_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FixtureCase {
    pub id: String,
    pub version: u32,
    pub seed: u64,
    pub payload: String,
}

impl FixtureCase {
    pub fn synthetic(
        id: impl Into<String>,
        version: u32,
        seed: u64,
        payload: impl Into<String>,
    ) -> io::Result<Self> {
        let case = Self {
            id: id.into(),
            version,
            seed,
            payload: payload.into(),
        };
        case.validate()?;
        Ok(case)
    }

    pub fn validate(&self) -> io::Result<()> {
        validate_fixture_id(&self.id)?;
        if self.payload.len() > MAX_PAYLOAD_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "fixture payload exceeds limit",
            ));
        }
        if self.payload.contains("-----BEGIN") || self.payload.contains("AKIA") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "fixture resembles a secret",
            ));
        }
        Ok(())
    }

    pub fn manifest_hash(&self) -> io::Result<String> {
        self.validate()?;
        let encoded = serde_json::to_vec(self)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        Ok(format!("{:016x}", fnv1a64(&encoded)))
    }
}

#[derive(Debug)]
pub struct FixtureWorkspace {
    root: PathBuf,
}

impl FixtureWorkspace {
    pub fn create(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn write(&self, case: &FixtureCase) -> io::Result<String> {
        let path = self.fixture_path(&case.id)?;
        let hash = case.manifest_hash()?;
        let encoded = serde_json::to_vec_pretty(case)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if encoded.len() > MAX_SERIALIZED_FIXTURE_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "fixture file exceeds serialized size limit",
            ));
        }
        self.write_fixture_file(&path, &encoded)?;
        Ok(hash)
    }

    pub fn read(&self, id: &str) -> io::Result<FixtureCase> {
        let path = self.fixture_path(id)?;
        let file = fs::File::open(path)?;
        if file.metadata()?.len() > MAX_SERIALIZED_FIXTURE_BYTES as u64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "fixture file exceeds serialized size limit",
            ));
        }
        let mut encoded = Vec::with_capacity(MAX_SERIALIZED_FIXTURE_BYTES.min(4096));
        file.take((MAX_SERIALIZED_FIXTURE_BYTES + 1) as u64)
            .read_to_end(&mut encoded)?;
        if encoded.len() > MAX_SERIALIZED_FIXTURE_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "fixture file exceeds serialized size limit",
            ));
        }
        let case: FixtureCase = serde_json::from_slice(&encoded)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        case.validate()?;
        Ok(case)
    }

    fn fixture_path(&self, id: &str) -> io::Result<PathBuf> {
        validate_fixture_id(id)?;
        let root = fs::canonicalize(&self.root)?;
        let path = root.join(format!("{id}.json"));
        if path.parent() != Some(root.as_path())
            || path
                .components()
                .any(|component| component == Component::ParentDir)
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "fixture path must remain inside workspace",
            ));
        }

        if fs::symlink_metadata(&path).is_ok() {
            let target = fs::canonicalize(&path)?;
            if target.parent() != Some(root.as_path()) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "fixture path escapes workspace",
                ));
            }
        }

        Ok(path)
    }

    fn write_fixture_file(&self, path: &Path, encoded: &[u8]) -> io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        file.write_all(encoded)
    }
}

impl Drop for FixtureWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn validate_fixture_id(id: &str) -> io::Result<()> {
    let is_safe_component = id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    if id.is_empty()
        || id.len() > MAX_FIXTURE_ID_BYTES
        || id == "."
        || id == ".."
        || !is_safe_component
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "fixture id must be a safe path component",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn fixture_is_deterministic_and_roundtrips() {
        let first = FixtureCase::synthetic("project-basic", 1, 42, "synthetic-project").unwrap();
        let second = FixtureCase::synthetic("project-basic", 1, 42, "synthetic-project").unwrap();
        assert_eq!(
            first.manifest_hash().unwrap(),
            second.manifest_hash().unwrap()
        );
        let directory = tempdir().unwrap();
        let workspace = FixtureWorkspace::create(directory.path().join("fixtures")).unwrap();
        let hash = workspace.write(&first).unwrap();
        assert_eq!(workspace.read("project-basic").unwrap(), first);
        assert_eq!(hash, first.manifest_hash().unwrap());
    }

    #[test]
    fn fixture_rejects_secrets_and_oversized_payloads() {
        assert!(FixtureCase::synthetic("secret", 1, 1, "AKIA1234567890123456").is_err());
        assert!(FixtureCase::synthetic("large", 1, 1, "x".repeat(MAX_PAYLOAD_BYTES + 1)).is_err());
    }

    #[test]
    fn fixture_rejects_path_like_ids() {
        for id in [
            ".",
            "..",
            "../outside",
            r"..\outside",
            "/outside",
            r"C:\outside",
        ] {
            assert!(
                FixtureCase::synthetic(id, 1, 1, "payload").is_err(),
                "path-like fixture id was accepted: {id}"
            );
        }
    }

    #[test]
    fn workspace_read_rejects_oversized_serialized_fixture() {
        let directory = tempdir().unwrap();
        let workspace = FixtureWorkspace::create(directory.path().join("fixtures")).unwrap();
        let path = workspace.fixture_path("oversized").unwrap();
        fs::write(&path, vec![b'x'; MAX_SERIALIZED_FIXTURE_BYTES + 1]).unwrap();

        let error = workspace.read("oversized").unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn workspace_roundtrips_escape_heavy_fixture_with_serialized_bound() {
        let directory = tempdir().unwrap();
        let workspace = FixtureWorkspace::create(directory.path().join("fixtures")).unwrap();
        let case =
            FixtureCase::synthetic("escape-heavy", 1, 7, "\"\\".repeat(MAX_PAYLOAD_BYTES / 2))
                .unwrap();

        let hash = workspace.write(&case).unwrap();

        assert_eq!(workspace.read("escape-heavy").unwrap(), case);
        assert_eq!(hash, case.manifest_hash().unwrap());
    }

    #[test]
    fn workspace_open_does_not_truncate_existing_fixture() {
        let directory = tempdir().unwrap();
        let workspace = FixtureWorkspace::create(directory.path().join("fixtures")).unwrap();
        let case = FixtureCase::synthetic("existing", 1, 1, "payload").unwrap();
        let path = workspace.fixture_path(&case.id).unwrap();
        fs::write(&path, "sentinel").unwrap();
        let encoded = serde_json::to_vec_pretty(&case).unwrap();

        let error = workspace.write_fixture_file(&path, &encoded).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(path).unwrap(), "sentinel");
    }

    #[cfg(unix)]
    #[test]
    fn workspace_open_rejects_symlink_replaced_after_path_validation() {
        let directory = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let outside_path = outside.path().join("outside.json");
        fs::write(&outside_path, "sentinel").unwrap();
        let workspace = FixtureWorkspace::create(directory.path().join("fixtures")).unwrap();
        let case = FixtureCase::synthetic("race", 1, 1, "payload").unwrap();
        let path = workspace.fixture_path(&case.id).unwrap();

        // Simulate an attacker replacing the validated path before the file
        // open. The exclusive open must reject the symlink instead of
        // following it to the outside file.
        std::os::unix::fs::symlink(&outside_path, &path).unwrap();
        let encoded = serde_json::to_vec_pretty(&case).unwrap();
        let error = workspace.write_fixture_file(&path, &encoded).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(outside_path).unwrap(), "sentinel");
    }

    #[test]
    fn workspace_cleans_up_on_drop() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("fixtures");
        {
            let workspace = FixtureWorkspace::create(&path).unwrap();
            workspace
                .write(&FixtureCase::synthetic("cleanup", 1, 7, "ok").unwrap())
                .unwrap();
        }
        assert!(!path.exists());
    }
}
