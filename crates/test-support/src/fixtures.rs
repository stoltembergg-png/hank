//! Deterministic, offline fixtures for contract and integration tests.
//!
//! This module is dev-only support: it creates synthetic bounded data, records a
//! manifest hash, and owns cleanup. It never reads production state or secrets.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const MAX_PAYLOAD_BYTES: usize = 64 * 1024;

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
        if self.id.is_empty() || self.id.len() > 128 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "fixture id must be bounded",
            ));
        }
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
        let hash = case.manifest_hash()?;
        let path = self.root.join(format!("{}.json", case.id));
        let encoded = serde_json::to_vec_pretty(case)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        fs::write(path, encoded)?;
        Ok(hash)
    }

    pub fn read(&self, id: &str) -> io::Result<FixtureCase> {
        let case: FixtureCase =
            serde_json::from_slice(&fs::read(self.root.join(format!("{id}.json")))?)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        case.validate()?;
        Ok(case)
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
