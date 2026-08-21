//! Bounded, deterministic, project-isolated registry for executable tools.

use crate::Tool;
use crate::schema::ToolSchemaError;
use agent_protocol::ids::{ProjectId, TraceId};
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

const DEFAULT_CAPACITY: usize = 256;
const MAX_CAPACITY: usize = 1024;
const MAX_IDENTIFIER_BYTES: usize = 128;

/// Registry lifecycle of a registered tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolLifecycle {
    Active,
    Disabled,
    Retired,
}

/// Scope in which a tool registration is visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolScope {
    Global,
    Project(ProjectId),
}

/// Authorized origin of a registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolOrigin {
    Builtin,
    Project(ProjectId),
    TrustedExtension(String),
}

/// Stable tool identity including project scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolIdentity {
    pub name: String,
    pub version: String,
    pub scope: ToolScope,
}

impl ToolIdentity {
    pub fn global(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            scope: ToolScope::Global,
        }
    }

    pub fn project(
        name: impl Into<String>,
        version: impl Into<String>,
        project: ProjectId,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            scope: ToolScope::Project(project),
        }
    }
}

/// Registration request; constructing it does not execute the tool.
pub struct ToolRegistrationRequest {
    pub tool: Arc<dyn Tool>,
    pub origin: ToolOrigin,
    pub scope: ToolScope,
    pub trace_id: TraceId,
}

impl ToolRegistrationRequest {
    pub fn new(
        tool: Arc<dyn Tool>,
        origin: ToolOrigin,
        scope: ToolScope,
        trace_id: TraceId,
    ) -> Self {
        Self {
            tool,
            origin,
            scope,
            trace_id,
        }
    }
}

/// Lookup request carrying project and optional capability requirements.
#[derive(Debug, Clone)]
pub struct ToolLookupRequest {
    pub name: String,
    pub version: String,
    pub project_id: ProjectId,
    pub capability: Option<String>,
    pub trace_id: TraceId,
}

impl ToolLookupRequest {
    pub fn new(
        name: String,
        version: String,
        project_id: ProjectId,
        capability: Option<String>,
        trace_id: TraceId,
    ) -> Self {
        Self {
            name,
            version,
            project_id,
            capability,
            trace_id,
        }
    }
}

/// Metadata returned by registry reads; no handler or untrusted description is exposed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDescriptor {
    pub name: String,
    pub version: String,
    pub origin: ToolOrigin,
    pub scope: ToolScope,
    pub lifecycle: ToolLifecycle,
    pub capabilities: Vec<String>,
    pub destructive: bool,
    pub environment: crate::ToolEnvironment,
}

/// Removed registration retained for bounded rollback/restore.
pub struct RemovedTool {
    request: ToolRegistrationRequest,
    identity: ToolIdentity,
    lifecycle: ToolLifecycle,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RegistryKey {
    name: String,
    version: String,
    scope: String,
}

impl RegistryKey {
    fn from_identity(identity: &ToolIdentity) -> Self {
        Self {
            name: identity.name.clone(),
            version: identity.version.clone(),
            scope: scope_key(identity.scope),
        }
    }
}

struct ToolEntry {
    request: ToolRegistrationRequest,
    identity: ToolIdentity,
    descriptor: ToolDescriptor,
}

struct RegistryState {
    entries: BTreeMap<RegistryKey, ToolEntry>,
    sealed: bool,
}

/// Registry errors. Error values contain bounded identities only.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistryError {
    #[error("duplicate tool identity: {name}@{version}")]
    DuplicateIdentity { name: String, version: String },
    #[error("tool not found: {name}@{version}")]
    NotFound { name: String, version: String },
    #[error("tool is not active: {name}@{version}")]
    NotActive { name: String, version: String },
    #[error("tool capability mismatch")]
    CapabilityMismatch,
    #[error("tool schema is invalid")]
    SchemaInvalid(#[source] ToolSchemaError),
    #[error("registration origin is not authorized for scope")]
    UnauthorizedOrigin,
    #[error("registry capacity reached")]
    Capacity,
    #[error("registry is sealed")]
    Sealed,
    #[error("registry request is invalid")]
    InvalidRequest,
}

/// Thread-safe bounded registry. It never executes a tool during registration or reads.
pub struct ToolRegistry {
    capacity: usize,
    state: RwLock<RegistryState>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            capacity: DEFAULT_CAPACITY,
            state: RwLock::new(RegistryState {
                entries: BTreeMap::new(),
                sealed: false,
            }),
        }
    }

    pub fn with_capacity(capacity: usize) -> Result<Self, RegistryError> {
        if capacity == 0 || capacity > MAX_CAPACITY {
            return Err(RegistryError::Capacity);
        }
        Ok(Self {
            capacity,
            state: RwLock::new(RegistryState {
                entries: BTreeMap::new(),
                sealed: false,
            }),
        })
    }

    /// Registers a validated tool without invoking its handler.
    pub fn register(&self, request: ToolRegistrationRequest) -> Result<(), RegistryError> {
        let schema = request.tool.schema();
        schema.validate().map_err(RegistryError::SchemaInvalid)?;
        if schema.name.trim().is_empty() || schema.version.trim().is_empty() {
            return Err(RegistryError::InvalidRequest);
        }
        validate_origin_scope(&request.origin, request.scope)?;
        let identity = ToolIdentity {
            name: schema.name.clone(),
            version: schema.version.clone(),
            scope: request.scope,
        };
        let key = RegistryKey::from_identity(&identity);
        let descriptor = ToolDescriptor {
            name: schema.name.clone(),
            version: schema.version.clone(),
            origin: request.origin.clone(),
            scope: request.scope,
            lifecycle: ToolLifecycle::Active,
            capabilities: schema.capabilities.clone(),
            destructive: schema.destructive,
            environment: schema.environment,
        };
        let mut state = self.state.write().map_err(|_| RegistryError::Sealed)?;
        if state.sealed {
            return Err(RegistryError::Sealed);
        }
        if state.entries.contains_key(&key) {
            return Err(RegistryError::DuplicateIdentity {
                name: identity.name,
                version: identity.version,
            });
        }
        if state.entries.len() >= self.capacity {
            return Err(RegistryError::Capacity);
        }
        state.entries.insert(
            key,
            ToolEntry {
                request,
                identity,
                descriptor,
            },
        );
        Ok(())
    }

    /// Resolves an active tool, preferring project scope over global scope.
    pub fn resolve(&self, request: &ToolLookupRequest) -> Result<Arc<dyn Tool>, RegistryError> {
        validate_lookup(request)?;
        let state = self.state.read().map_err(|_| RegistryError::Sealed)?;
        let candidates = [
            RegistryKey {
                name: request.name.clone(),
                version: request.version.clone(),
                scope: scope_key(ToolScope::Project(request.project_id)),
            },
            RegistryKey {
                name: request.name.clone(),
                version: request.version.clone(),
                scope: scope_key(ToolScope::Global),
            },
        ];
        for key in candidates {
            let Some(entry) = state.entries.get(&key) else {
                continue;
            };
            if entry.descriptor.lifecycle != ToolLifecycle::Active {
                return Err(RegistryError::NotActive {
                    name: request.name.clone(),
                    version: request.version.clone(),
                });
            }
            if let Some(capability) = &request.capability
                && !entry.descriptor.capabilities.contains(capability)
            {
                return Err(RegistryError::CapabilityMismatch);
            }
            return Ok(entry.request.tool.clone());
        }
        Err(RegistryError::NotFound {
            name: request.name.clone(),
            version: request.version.clone(),
        })
    }

    pub fn descriptor(&self, identity: &ToolIdentity) -> Result<ToolDescriptor, RegistryError> {
        let state = self.state.read().map_err(|_| RegistryError::Sealed)?;
        state
            .entries
            .get(&RegistryKey::from_identity(identity))
            .map(|entry| entry.descriptor.clone())
            .ok_or_else(|| RegistryError::NotFound {
                name: identity.name.clone(),
                version: identity.version.clone(),
            })
    }

    pub fn list_all(&self) -> Result<Vec<ToolDescriptor>, RegistryError> {
        let state = self.state.read().map_err(|_| RegistryError::Sealed)?;
        Ok(state
            .entries
            .values()
            .map(|entry| entry.descriptor.clone())
            .collect())
    }

    pub fn list_visible(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<ToolDescriptor>, RegistryError> {
        let state = self.state.read().map_err(|_| RegistryError::Sealed)?;
        Ok(state
            .entries
            .values()
            .filter(|entry| {
                entry.descriptor.scope == ToolScope::Global
                    || entry.descriptor.scope == ToolScope::Project(*project_id)
            })
            .map(|entry| entry.descriptor.clone())
            .collect())
    }

    pub fn list_by_capability(
        &self,
        project_id: &ProjectId,
        capability: &str,
    ) -> Result<Vec<ToolDescriptor>, RegistryError> {
        if !valid_bounded(capability) {
            return Err(RegistryError::InvalidRequest);
        }
        Ok(self
            .list_visible(project_id)?
            .into_iter()
            .filter(|descriptor| {
                descriptor
                    .capabilities
                    .iter()
                    .any(|item| item == capability)
            })
            .collect())
    }

    pub fn set_lifecycle(
        &self,
        identity: &ToolIdentity,
        lifecycle: ToolLifecycle,
    ) -> Result<(), RegistryError> {
        let mut state = self.state.write().map_err(|_| RegistryError::Sealed)?;
        if state.sealed {
            return Err(RegistryError::Sealed);
        }
        let entry = state
            .entries
            .get_mut(&RegistryKey::from_identity(identity))
            .ok_or_else(|| RegistryError::NotFound {
                name: identity.name.clone(),
                version: identity.version.clone(),
            })?;
        entry.descriptor.lifecycle = lifecycle;
        Ok(())
    }

    pub fn unregister(&self, identity: &ToolIdentity) -> Result<RemovedTool, RegistryError> {
        let mut state = self.state.write().map_err(|_| RegistryError::Sealed)?;
        if state.sealed {
            return Err(RegistryError::Sealed);
        }
        let entry = state
            .entries
            .remove(&RegistryKey::from_identity(identity))
            .ok_or_else(|| RegistryError::NotFound {
                name: identity.name.clone(),
                version: identity.version.clone(),
            })?;
        Ok(RemovedTool {
            request: entry.request,
            identity: entry.identity,
            lifecycle: entry.descriptor.lifecycle,
        })
    }

    pub fn restore(&self, removed: RemovedTool) -> Result<(), RegistryError> {
        let lifecycle = removed.lifecycle;
        let identity = removed.identity.clone();
        self.register(removed.request)?;
        if lifecycle != ToolLifecycle::Active {
            self.set_lifecycle(&identity, lifecycle)?;
        }
        Ok(())
    }

    pub fn seal(&self) -> Result<(), RegistryError> {
        let mut state = self.state.write().map_err(|_| RegistryError::Sealed)?;
        state.sealed = true;
        Ok(())
    }

    pub fn is_sealed(&self) -> bool {
        self.state.read().map(|state| state.sealed).unwrap_or(true)
    }

    pub fn len(&self) -> Result<usize, RegistryError> {
        Ok(self
            .state
            .read()
            .map_err(|_| RegistryError::Sealed)?
            .entries
            .len())
    }

    pub fn is_empty(&self) -> Result<bool, RegistryError> {
        Ok(self.len()? == 0)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_origin_scope(origin: &ToolOrigin, scope: ToolScope) -> Result<(), RegistryError> {
    match (origin, scope) {
        (ToolOrigin::Builtin, ToolScope::Global) => Ok(()),
        (ToolOrigin::Project(origin_project), ToolScope::Project(scope_project))
            if origin_project == &scope_project =>
        {
            Ok(())
        }
        (ToolOrigin::TrustedExtension(extension), _) if valid_bounded(extension) => Ok(()),
        _ => Err(RegistryError::UnauthorizedOrigin),
    }
}

fn validate_lookup(request: &ToolLookupRequest) -> Result<(), RegistryError> {
    if !valid_bounded(&request.name)
        || !valid_bounded(&request.version)
        || request
            .capability
            .as_deref()
            .is_some_and(|capability| !valid_bounded(capability))
    {
        return Err(RegistryError::InvalidRequest);
    }
    Ok(())
}

fn valid_bounded(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && !value.chars().any(char::is_control)
        && !value.contains("..")
        && !value.contains('/')
        && !value.contains('\\')
}

fn scope_key(scope: ToolScope) -> String {
    match scope {
        ToolScope::Global => "global".to_string(),
        ToolScope::Project(project_id) => format!("project:{project_id}"),
    }
}
