//! Non-executing MCP tool discovery and staging contract.
use std::collections::BTreeSet;
const MAX: usize = 256;
const MAX_TOOLS: usize = 128;
const MAX_SCHEMA: usize = 64 * 1024;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDescription {
    name: String,
    schema_size: usize,
}
impl ToolDescription {
    pub fn new(name: &str, schema_size: usize) -> Self {
        Self {
            name: name.into(),
            schema_size,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolManifest {
    revision: String,
    capabilities: Vec<String>,
    tools: Vec<ToolDescription>,
}
impl ToolManifest {
    pub fn new(
        revision: &str,
        capabilities: Vec<String>,
        tools: Vec<ToolDescription>,
    ) -> Result<Self, DiscoveryError> {
        if revision.is_empty()
            || revision.len() > MAX
            || capabilities.iter().any(|c| c.is_empty() || c.len() > MAX)
            || tools.len() > MAX_TOOLS
        {
            return Err(DiscoveryError::ManifestInvalid);
        }
        let mut names = BTreeSet::new();
        for tool in &tools {
            if tool.name.is_empty()
                || tool.name.len() > MAX
                || tool.schema_size == 0
                || tool.schema_size > MAX_SCHEMA
                || !names.insert(tool.name.clone())
            {
                return Err(DiscoveryError::DuplicateTool);
            }
        }
        Ok(Self {
            revision: revision.into(),
            capabilities,
            tools,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryRequest {
    server: String,
    transport_authorized: bool,
    server_authorized: bool,
    max_manifest: usize,
    capabilities: Vec<String>,
}
impl DiscoveryRequest {
    pub fn new(
        server: &str,
        transport_authorized: bool,
        server_authorized: bool,
        max_manifest: usize,
        capabilities: Vec<String>,
    ) -> Result<Self, DiscoveryError> {
        if server.is_empty() || server.len() > MAX || max_manifest == 0 || max_manifest > MAX_SCHEMA
        {
            return Err(DiscoveryError::RequestInvalid);
        }
        Ok(Self {
            server: server.into(),
            transport_authorized,
            server_authorized,
            max_manifest,
            capabilities,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryState {
    Pending,
    Disabled,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedEntry {
    name: String,
    schema_size: usize,
    state: EntryState,
}
impl StagedEntry {
    pub fn state(&self) -> EntryState {
        self.state
    }
    pub fn execution_enabled(&self) -> bool {
        false
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryError {
    RequestInvalid,
    ManifestInvalid,
    TransportUnauthorized,
    ServerUnauthorized,
    CapabilityDenied,
    DuplicateTool,
    ManifestTooLarge,
    RevisionStale,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryResult {
    revision: String,
    entries: Vec<StagedEntry>,
    max_manifest: usize,
}
impl DiscoveryResult {
    pub fn entries(&self) -> &[StagedEntry] {
        &self.entries
    }
    pub fn refresh(
        self,
        revision: &str,
        tools: Vec<ToolDescription>,
    ) -> Result<Self, DiscoveryError> {
        if revision.is_empty() || revision == self.revision {
            return Err(DiscoveryError::RevisionStale);
        }
        let manifest = ToolManifest::new(revision, Vec::new(), tools)?;
        let entries = stage(manifest.tools);
        if entries.iter().map(|e| e.schema_size).sum::<usize>() > self.max_manifest {
            return Err(DiscoveryError::ManifestTooLarge);
        }
        Ok(Self {
            revision: manifest.revision,
            entries,
            max_manifest: self.max_manifest,
        })
    }
}
pub struct Discovery;
impl Discovery {
    pub fn process(
        request: DiscoveryRequest,
        manifest: ToolManifest,
    ) -> Result<DiscoveryResult, DiscoveryError> {
        if !request.transport_authorized {
            return Err(DiscoveryError::TransportUnauthorized);
        }
        if !request.server_authorized {
            return Err(DiscoveryError::ServerUnauthorized);
        }
        if manifest
            .capabilities
            .iter()
            .any(|c| !request.capabilities.contains(c))
        {
            return Err(DiscoveryError::CapabilityDenied);
        }
        if manifest.tools.iter().map(|t| t.schema_size).sum::<usize>() > request.max_manifest {
            return Err(DiscoveryError::ManifestTooLarge);
        }
        Ok(DiscoveryResult {
            revision: manifest.revision,
            entries: stage(manifest.tools),
            max_manifest: request.max_manifest,
        })
    }
}
fn stage(mut tools: Vec<ToolDescription>) -> Vec<StagedEntry> {
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    tools
        .into_iter()
        .map(|t| StagedEntry {
            name: t.name,
            schema_size: t.schema_size,
            state: EntryState::Pending,
        })
        .collect()
}
