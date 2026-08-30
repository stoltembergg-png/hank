use crate::PluginManifest;
use std::collections::BTreeSet;
use thiserror::Error;

const MAX_SOURCE: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginStage {
    Staged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedPlugin {
    source: String,
    manifest: PluginManifest,
    stage: PluginStage,
}

impl StagedPlugin {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    pub fn stage(&self) -> PluginStage {
        self.stage
    }

    pub fn execution_enabled(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginCatalog {
    entries: Vec<StagedPlugin>,
}

impl PluginCatalog {
    pub fn entries(&self) -> &[StagedPlugin] {
        &self.entries
    }

    pub fn ids(&self) -> Vec<&str> {
        self.entries
            .iter()
            .map(|entry| entry.manifest().id())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DiscoveryError {
    #[error("invalid discovery root")]
    InvalidRoot,
    #[error("source is outside the allowlist")]
    SourceNotAllowed,
    #[error("discovery input exceeds the limit")]
    TooManyEntries,
    #[error("plugin ID is duplicated")]
    DuplicatePlugin,
    #[error("plugin API is unsupported")]
    ApiUnsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginDiscovery {
    root: String,
    max_entries: usize,
}

impl PluginDiscovery {
    pub fn new(root: &str, max_entries: usize) -> Result<Self, DiscoveryError> {
        if root.is_empty() || root.len() > MAX_SOURCE || max_entries == 0 {
            return Err(DiscoveryError::InvalidRoot);
        }
        Ok(Self {
            root: root.trim_end_matches('/').to_owned(),
            max_entries,
        })
    }

    pub fn discover(
        &self,
        candidates: Vec<(&str, PluginManifest)>,
    ) -> Result<PluginCatalog, DiscoveryError> {
        if candidates.len() > self.max_entries {
            return Err(DiscoveryError::TooManyEntries);
        }
        let mut ids = BTreeSet::new();
        let mut entries = Vec::with_capacity(candidates.len());
        for (source, manifest) in candidates {
            if !self.is_allowed_source(source) {
                return Err(DiscoveryError::SourceNotAllowed);
            }
            if manifest.api_revision() != "api-1" {
                return Err(DiscoveryError::ApiUnsupported);
            }
            if !ids.insert(manifest.id().to_owned()) {
                return Err(DiscoveryError::DuplicatePlugin);
            }
            entries.push(StagedPlugin {
                source: source.to_owned(),
                manifest,
                stage: PluginStage::Staged,
            });
        }
        entries.sort_by(|left, right| left.manifest().id().cmp(right.manifest().id()));
        Ok(PluginCatalog { entries })
    }

    fn is_allowed_source(&self, source: &str) -> bool {
        source.len() <= MAX_SOURCE
            && (source == self.root
                || source
                    .strip_prefix(&self.root)
                    .is_some_and(|suffix| suffix.starts_with('/')))
    }
}
