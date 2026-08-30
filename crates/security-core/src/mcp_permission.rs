//! Provider-neutral MCP permission policy with default deny.
use std::collections::BTreeSet;
const MAX: usize = 128;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionAction {
    Discovery,
    Execution,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecision {
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
pub struct PermissionRequest {
    pub request_id: u64,
    pub policy_revision: String,
    pub server: String,
    pub tool: String,
    pub origin: String,
    pub project: String,
    pub agent: String,
    pub action: PermissionAction,
}
impl PermissionRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u64,
        policy: &str,
        server: &str,
        tool: &str,
        origin: &str,
        project: &str,
        agent: &str,
        action: PermissionAction,
    ) -> Self {
        Self {
            request_id: id,
            policy_revision: policy.into(),
            server: server.into(),
            tool: tool.into(),
            origin: origin.into(),
            project: project.into(),
            agent: agent.into(),
            action,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grant {
    server: String,
    tool: String,
    origin: String,
    project: String,
    agent: String,
    action: PermissionAction,
    duration: GrantDuration,
    expires_at: u64,
    used: bool,
}
impl Grant {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        server: &str,
        tool: &str,
        origin: &str,
        project: &str,
        agent: &str,
        action: PermissionAction,
        duration: GrantDuration,
        expires_at: u64,
    ) -> Result<Self, PermissionError> {
        if [server, tool, origin, project, agent]
            .iter()
            .any(|v| v.is_empty() || v.len() > MAX)
            || expires_at == 0
        {
            return Err(PermissionError::InvalidGrant);
        }
        Ok(Self {
            server: server.into(),
            tool: tool.into(),
            origin: origin.into(),
            project: project.into(),
            agent: agent.into(),
            action,
            duration,
            expires_at,
            used: false,
        })
    }
    fn matches(&self, r: &PermissionRequest, now: u64) -> bool {
        !self.used
            && self.expires_at > now
            && self.server == r.server
            && self.tool == r.tool
            && self.origin == r.origin
            && self.project == r.project
            && self.agent == r.agent
            && self.action == r.action
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionError {
    InvalidPolicy,
    InvalidGrant,
    PolicyStale,
    Replay,
    RevocationUnknown,
}
pub struct PermissionEngine {
    policy: String,
    grants: Vec<Grant>,
    seen: BTreeSet<u64>,
}
impl PermissionEngine {
    pub fn new(policy: &str) -> Result<Self, PermissionError> {
        if policy.is_empty() || policy.len() > MAX {
            return Err(PermissionError::InvalidPolicy);
        }
        Ok(Self {
            policy: policy.into(),
            grants: Vec::new(),
            seen: BTreeSet::new(),
        })
    }
    pub fn grant(&mut self, grant: Grant) -> Result<(), PermissionError> {
        self.grants.push(grant);
        Ok(())
    }
    pub fn evaluate(
        &mut self,
        r: PermissionRequest,
        now: u64,
    ) -> Result<PermissionDecision, PermissionError> {
        if r.policy_revision != self.policy {
            return Err(PermissionError::PolicyStale);
        }
        if !self.seen.insert(r.request_id) {
            return Err(PermissionError::Replay);
        }
        if let Some(grant) = self.grants.iter_mut().find(|g| g.matches(&r, now)) {
            if grant.duration == GrantDuration::OneShot {
                grant.used = true
            }
            return Ok(PermissionDecision::Allowed);
        }
        Ok(PermissionDecision::Denied)
    }
    pub fn revoke(&mut self, server: &str, tool: &str) -> Result<(), PermissionError> {
        let mut found = false;
        for grant in &mut self.grants {
            if grant.server == server && grant.tool == tool {
                grant.used = true;
                found = true
            }
        }
        if found {
            Ok(())
        } else {
            Err(PermissionError::RevocationUnknown)
        }
    }
}
