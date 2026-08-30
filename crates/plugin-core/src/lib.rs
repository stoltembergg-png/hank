//! Canonical, bounded plugin manifest contract.
use semver::Version;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
const MAX: usize = 256;
const MAX_LIST: usize = 64;
const MAX_DEPS: usize = 32;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Isolation {
    Process,
    Wasm,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustState {
    Trusted,
    Untrusted,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginManifest {
    id: String,
    version: Version,
    api_revision: String,
    entrypoint: String,
    capabilities: Vec<String>,
    os: Vec<String>,
    dependencies: Vec<String>,
    signer: String,
    provenance: String,
    isolation: Isolation,
    trust: TrustState,
}
impl PluginManifest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        version: &str,
        api: &str,
        entrypoint: &str,
        capabilities: Vec<String>,
        os: Vec<String>,
        dependencies: Vec<String>,
        signer: &str,
        provenance: &str,
        isolation: Isolation,
    ) -> Result<Self, ManifestError> {
        if [id, api, entrypoint]
            .iter()
            .any(|v| v.is_empty() || v.len() > MAX)
            || capabilities.len() > MAX_LIST
            || os.len() > MAX_LIST
            || dependencies.len() > MAX_DEPS
        {
            return Err(ManifestError::InvalidField);
        }
        let version = Version::parse(version).map_err(|_| ManifestError::InvalidVersion)?;
        if api.is_empty() {
            return Err(ManifestError::InvalidApiRevision);
        }
        let allowed = BTreeSet::from(["read", "write", "network", "process"]);
        if capabilities.iter().any(|c| !allowed.contains(c.as_str())) {
            return Err(ManifestError::CapabilityDenied);
        }
        if [id, api, entrypoint, signer, provenance].iter().any(|v| {
            let l = v.to_ascii_lowercase();
            l.contains("token") || l.contains("secret") || l.contains("password")
        }) {
            return Err(ManifestError::SecretLike);
        }
        let trust = if signer.is_empty() || provenance.is_empty() {
            TrustState::Untrusted
        } else {
            TrustState::Trusted
        };
        Ok(Self {
            id: id.into(),
            version,
            api_revision: api.into(),
            entrypoint: entrypoint.into(),
            capabilities,
            os,
            dependencies,
            signer: signer.into(),
            provenance: provenance.into(),
            isolation,
            trust,
        })
    }
    pub fn trust(&self) -> TrustState {
        self.trust
    }
    pub fn digest(&self) -> String {
        let mut h = 0xcbf29ce484222325u64;
        for b in self.canonical().as_bytes() {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x100000001b3)
        }
        format!("{h:016x}")
    }
    fn canonical(&self) -> String {
        format!(
            "{}|{}|{}|{}|{:?}|{:?}|{:?}|{}|{}|{:?}",
            self.id,
            self.version,
            self.api_revision,
            self.entrypoint,
            self.capabilities,
            self.os,
            self.dependencies,
            self.signer,
            self.provenance,
            self.isolation
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ManifestError {
    #[error("invalid manifest field")]
    InvalidField,
    #[error("invalid semantic version")]
    InvalidVersion,
    #[error("invalid API revision")]
    InvalidApiRevision,
    #[error("capability denied")]
    CapabilityDenied,
    #[error("secret-like value forbidden")]
    SecretLike,
    #[error("dependency cycle")]
    DependencyCycle,
}
pub struct DependencyGraph;
impl DependencyGraph {
    pub fn validate(manifests: &[PluginManifest]) -> Result<(), ManifestError> {
        let map: BTreeMap<&str, &[String]> = manifests
            .iter()
            .map(|m| (m.id.as_str(), m.dependencies.as_slice()))
            .collect();
        let mut visiting = BTreeSet::new();
        let mut done = BTreeSet::new();
        for id in map.keys() {
            if !visit(id, &map, &mut visiting, &mut done) {
                return Err(ManifestError::DependencyCycle);
            }
        }
        Ok(())
    }
}
fn visit(
    id: &str,
    map: &BTreeMap<&str, &[String]>,
    visiting: &mut BTreeSet<String>,
    done: &mut BTreeSet<String>,
) -> bool {
    if done.contains(id) {
        return true;
    }
    if !visiting.insert(id.into()) {
        return false;
    }
    if let Some(deps) = map.get(id) {
        for dep in (*deps).iter() {
            if map.contains_key(dep.as_str()) && !visit(dep, map, visiting, done) {
                return false;
            }
        }
    }
    visiting.remove(id);
    done.insert(id.into());
    true
}
