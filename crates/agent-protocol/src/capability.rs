//! Definição de capabilities e permissões do sistema.
//!
//! Capabilities são ações específicas que podem ser autorizadas ou negadas
//! pelo Permission Engine. Cada capability carrega seu recurso, ação
//! e escopo de projeto.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Capability granular: recurso + ação + escopo opcional
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Capability {
    pub resource: Resource,
    pub action: Action,
    pub scope: Option<String>,
}

/// Recursos protegidos pelo sistema de permissão
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resource {
    Project,
    Agent,
    Session,
    Message,
    Memory,
    Skill,
    Tool,
    Workflow,
    File,
    Process,
    Network,
    Secret,
    Provider,
    Plugin,
    RemoteNode,
    Settings,
}

/// Ações permitidas sobre recursos
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Create,
    Read,
    Update,
    Delete,
    List,
    Execute,
    Invoke,
    Delegate,
    Approve,
    Revoke,
    Configure,
    Discover,
    Stream,
    Cancel,
    Retry,
}

impl Capability {
    pub fn new(resource: Resource, action: Action) -> Self {
        Self {
            resource,
            action,
            scope: None,
        }
    }

    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(scope) = &self.scope {
            write!(f, "{}:{}:{}", self.resource, self.action, scope)
        } else {
            write!(f, "{}:{}", self.resource, self.action)
        }
    }
}

impl fmt::Display for Resource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Resource::Project => "project",
            Resource::Agent => "agent",
            Resource::Session => "session",
            Resource::Message => "message",
            Resource::Memory => "memory",
            Resource::Skill => "skill",
            Resource::Tool => "tool",
            Resource::Workflow => "workflow",
            Resource::File => "file",
            Resource::Process => "process",
            Resource::Network => "network",
            Resource::Secret => "secret",
            Resource::Provider => "provider",
            Resource::Plugin => "plugin",
            Resource::RemoteNode => "remote_node",
            Resource::Settings => "settings",
        };
        write!(f, "{}", s)
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Action::Create => "create",
            Action::Read => "read",
            Action::Update => "update",
            Action::Delete => "delete",
            Action::List => "list",
            Action::Execute => "execute",
            Action::Invoke => "invoke",
            Action::Delegate => "delegate",
            Action::Approve => "approve",
            Action::Revoke => "revoke",
            Action::Configure => "configure",
            Action::Discover => "discover",
            Action::Stream => "stream",
            Action::Cancel => "cancel",
            Action::Retry => "retry",
        };
        write!(f, "{}", s)
    }
}

/// Conjunto de capabilities para políticas compostas
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitySet {
    pub capabilities: Vec<Capability>,
}

impl CapabilitySet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(mut self, cap: Capability) -> Self {
        self.capabilities.push(cap);
        self
    }

    pub fn contains(&self, cap: &Capability) -> bool {
        self.capabilities.iter().any(|c| c == cap)
    }
}
