//! Explicit, provider-neutral plugin capability authorization.
use std::collections::BTreeSet;
use thiserror::Error;

const MAX: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginAction {
    Install,
    Start,
    Use,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginPermissionDecision {
    Allowed,
    Denied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantDuration {
    OneShot,
    Session,
    Persistent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginPermissionRequest {
    pub request_id: u64,
    pub policy_revision: String,
    pub plugin_id: String,
    pub digest: String,
    pub version: String,
    pub capability: String,
    pub project: String,
    pub agent: String,
    pub action: PluginAction,
}

impl PluginPermissionRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request_id: u64,
        policy_revision: &str,
        plugin_id: &str,
        digest: &str,
        version: &str,
        capability: &str,
        project: &str,
        agent: &str,
        action: PluginAction,
    ) -> Self {
        Self {
            request_id,
            policy_revision: policy_revision.into(),
            plugin_id: plugin_id.into(),
            digest: digest.into(),
            version: version.into(),
            capability: capability.into(),
            project: project.into(),
            agent: agent.into(),
            action,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginGrant {
    plugin_id: String,
    digest: String,
    version: String,
    capability: String,
    project: String,
    agent: String,
    duration: GrantDuration,
    revoked: bool,
    used: bool,
}

impl PluginGrant {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        plugin_id: &str,
        digest: &str,
        version: &str,
        capability: &str,
        project: &str,
        agent: &str,
        duration: GrantDuration,
    ) -> Result<Self, PluginPermissionError> {
        if [plugin_id, digest, version, capability, project, agent]
            .iter()
            .any(|value| value.is_empty() || value.len() > MAX)
            || !matches!(capability, "read" | "write" | "network" | "process")
        {
            return Err(PluginPermissionError::InvalidGrant);
        }
        Ok(Self {
            plugin_id: plugin_id.into(),
            digest: digest.into(),
            version: version.into(),
            capability: capability.into(),
            project: project.into(),
            agent: agent.into(),
            duration,
            revoked: false,
            used: false,
        })
    }

    fn matches(&self, request: &PluginPermissionRequest) -> bool {
        !self.revoked
            && !self.used
            && self.plugin_id == request.plugin_id
            && self.digest == request.digest
            && self.version == request.version
            && self.capability == request.capability
            && self.project == request.project
            && self.agent == request.agent
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PluginPermissionError {
    #[error("invalid policy")]
    InvalidPolicy,
    #[error("invalid grant")]
    InvalidGrant,
    #[error("policy revision is stale")]
    PolicyStale,
    #[error("request replayed")]
    Replay,
    #[error("grant not found")]
    GrantNotFound,
}

pub struct PluginPermissionEngine {
    policy_revision: String,
    grants: Vec<PluginGrant>,
    seen: BTreeSet<u64>,
}

impl PluginPermissionEngine {
    pub fn new(policy_revision: &str) -> Result<Self, PluginPermissionError> {
        if policy_revision.is_empty() || policy_revision.len() > MAX {
            return Err(PluginPermissionError::InvalidPolicy);
        }
        Ok(Self {
            policy_revision: policy_revision.into(),
            grants: Vec::new(),
            seen: BTreeSet::new(),
        })
    }

    pub fn grant(&mut self, grant: PluginGrant) -> Result<(), PluginPermissionError> {
        self.grants.push(grant);
        Ok(())
    }

    pub fn evaluate(
        &mut self,
        request: PluginPermissionRequest,
    ) -> Result<PluginPermissionDecision, PluginPermissionError> {
        if request.policy_revision != self.policy_revision {
            return Err(PluginPermissionError::PolicyStale);
        }
        if !self.seen.insert(request.request_id) {
            return Err(PluginPermissionError::Replay);
        }
        if let Some(grant) = self.grants.iter_mut().find(|grant| grant.matches(&request)) {
            if grant.duration == GrantDuration::OneShot {
                grant.used = true;
            }
            return Ok(PluginPermissionDecision::Allowed);
        }
        Ok(PluginPermissionDecision::Denied)
    }

    pub fn revoke(
        &mut self,
        plugin_id: &str,
        digest: &str,
        version: &str,
        capability: &str,
    ) -> Result<(), PluginPermissionError> {
        let mut found = false;
        for grant in &mut self.grants {
            if grant.plugin_id == plugin_id
                && grant.digest == digest
                && grant.version == version
                && grant.capability == capability
            {
                grant.revoked = true;
                found = true;
            }
        }
        if found {
            Ok(())
        } else {
            Err(PluginPermissionError::GrantNotFound)
        }
    }
}
