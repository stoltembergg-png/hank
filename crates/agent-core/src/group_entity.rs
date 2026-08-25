//! Project-scoped, non-executable AgentGroup domain entity.

use crate::{AgentId, BudgetLimits, DomainError, ProjectId, TraceId};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

pub const AGENT_GROUP_SCHEMA_VERSION: u32 = 1;
pub const MAX_GROUP_NAME_BYTES: usize = 64;
pub const MAX_GROUP_MEMBERS: usize = 32;
pub const MAX_CONTEXT_REFS: usize = 64;
pub const MAX_CONTEXT_REF_BYTES: usize = 256;
pub const MAX_GROUP_ROUNDS: u32 = 100;
pub const MAX_GROUP_DEPTH: u16 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentGroupLifecycle {
    Draft,
    Active,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AgentGroupError {
    #[error("group membership operation is not authorized")]
    MembershipPermissionDenied,
    #[error("group member is already present")]
    DuplicateMember,
    #[error("group member is not present")]
    MissingMember,
    #[error("membership role is invalid")]
    InvalidRole,
    #[error("membership snapshot is invalid")]
    InvalidMembershipSnapshot,
    #[error("group is archived")]
    Archived,
    #[error("group member project is unknown at entity boundary")]
    MemberProjectUnknown,
    #[error("group limits are invalid")]
    InvalidLimits,
    #[error("shared context reference is invalid")]
    InvalidContextReference,
    #[error("group must pin a version before activation")]
    MissingPinnedVersion,
    #[error("group name is invalid")]
    InvalidName,
    #[error("group trace is invalid")]
    InvalidTrace,
    #[error("group project identity is invalid")]
    InvalidProject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentGroupMembership {
    pub agent_id: AgentId,
    pub project_id: ProjectId,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentGroup {
    pub schema_version: u32,
    pub id: uuid::Uuid,
    pub project_id: ProjectId,
    pub name: String,
    pub owner_id: AgentId,
    pub members: Vec<AgentId>,
    pub member_projects: Vec<(AgentId, ProjectId)>,
    pub memberships: Vec<AgentGroupMembership>,
    pub moderator_id: Option<AgentId>,
    pub max_rounds: u32,
    pub max_depth: u16,
    pub allow_cycles: bool,
    pub budget: BudgetLimits,
    pub context_refs: Vec<String>,
    pub lifecycle: AgentGroupLifecycle,
    pub pinned_version: Option<String>,
    pub trace_id: TraceId,
}

impl AgentGroup {
    pub fn new(project_id: ProjectId, name: String, owner_id: AgentId, trace_id: TraceId) -> Self {
        Self {
            schema_version: AGENT_GROUP_SCHEMA_VERSION,
            id: uuid::Uuid::new_v4(),
            project_id,
            name,
            owner_id,
            members: vec![owner_id],
            member_projects: vec![(owner_id, project_id)],
            memberships: vec![AgentGroupMembership {
                agent_id: owner_id,
                project_id,
                role: "owner".into(),
            }],
            moderator_id: Some(owner_id),
            max_rounds: 20,
            max_depth: 8,
            allow_cycles: false,
            budget: BudgetLimits::default(),
            context_refs: Vec::new(),
            lifecycle: AgentGroupLifecycle::Draft,
            pinned_version: None,
            trace_id,
        }
    }

    pub fn validate(&self) -> Result<(), AgentGroupError> {
        if self.schema_version != AGENT_GROUP_SCHEMA_VERSION
            || self.project_id.to_string().is_empty()
        {
            return Err(AgentGroupError::InvalidProject);
        }
        if self.name.trim().is_empty() || self.name.len() > MAX_GROUP_NAME_BYTES {
            return Err(AgentGroupError::InvalidName);
        }
        if self.trace_id.as_uuid().is_nil() {
            return Err(AgentGroupError::InvalidTrace);
        }
        if self.members.is_empty() || self.members.len() > MAX_GROUP_MEMBERS {
            return Err(AgentGroupError::InvalidLimits);
        }
        let unique: HashSet<_> = self.members.iter().collect();
        if unique.len() != self.members.len()
            || self.member_projects.len() != self.members.len()
            || self
                .member_projects
                .iter()
                .any(|(member, project)| !unique.contains(&member) || *project != self.project_id)
            || self.moderator_id.is_some_and(|id| !unique.contains(&id))
        {
            return Err(AgentGroupError::MemberProjectUnknown);
        }
        if self.memberships.len() != self.members.len()
            || self.memberships.iter().any(|membership| {
                membership.project_id != self.project_id
                    || membership.role.trim().is_empty()
                    || membership.role.len() > 32
                    || !unique.contains(&membership.agent_id)
            })
        {
            return Err(AgentGroupError::InvalidMembershipSnapshot);
        }
        if self.max_rounds == 0
            || self.max_rounds > MAX_GROUP_ROUNDS
            || self.max_depth == 0
            || self.max_depth > MAX_GROUP_DEPTH
        {
            return Err(AgentGroupError::InvalidLimits);
        }
        self.budget
            .validate()
            .map_err(|_| AgentGroupError::InvalidLimits)?;
        if self.context_refs.len() > MAX_CONTEXT_REFS
            || self.context_refs.iter().any(|reference| {
                reference.len() > MAX_CONTEXT_REF_BYTES
                    || !reference.starts_with("project://")
                    || reference.contains("..")
            })
        {
            return Err(AgentGroupError::InvalidContextReference);
        }
        if self.lifecycle == AgentGroupLifecycle::Active && self.pinned_version.is_none() {
            return Err(AgentGroupError::MissingPinnedVersion);
        }
        Ok(())
    }

    pub fn activate(&mut self) -> Result<(), AgentGroupError> {
        if self.pinned_version.is_none() {
            return Err(AgentGroupError::MissingPinnedVersion);
        }
        self.validate()?;
        self.lifecycle = AgentGroupLifecycle::Active;
        Ok(())
    }

    pub fn archive(&mut self) -> Result<bool, AgentGroupError> {
        self.validate()?;
        if self.lifecycle == AgentGroupLifecycle::Archived {
            return Ok(false);
        }
        self.lifecycle = AgentGroupLifecycle::Archived;
        Ok(true)
    }

    pub fn add_member(
        &mut self,
        agent_id: AgentId,
        member_project: ProjectId,
        actor_id: AgentId,
        role: String,
    ) -> Result<(), AgentGroupError> {
        if self.lifecycle == AgentGroupLifecycle::Archived {
            return Err(AgentGroupError::Archived);
        }
        if actor_id != self.owner_id && self.moderator_id != Some(actor_id) {
            return Err(AgentGroupError::MembershipPermissionDenied);
        }
        if member_project != self.project_id {
            return Err(AgentGroupError::MemberProjectUnknown);
        }
        if role.trim().is_empty() || role.len() > 32 {
            return Err(AgentGroupError::InvalidRole);
        }
        if self.members.contains(&agent_id) {
            return Err(AgentGroupError::DuplicateMember);
        }
        if self.members.len() >= MAX_GROUP_MEMBERS {
            return Err(AgentGroupError::InvalidLimits);
        }
        self.members.push(agent_id);
        self.member_projects.push((agent_id, member_project));
        self.memberships.push(AgentGroupMembership {
            agent_id,
            project_id: member_project,
            role,
        });
        self.validate()
    }

    pub fn remove_member(
        &mut self,
        agent_id: AgentId,
        actor_id: AgentId,
    ) -> Result<(), AgentGroupError> {
        if actor_id != self.owner_id && self.moderator_id != Some(actor_id) {
            return Err(AgentGroupError::MembershipPermissionDenied);
        }
        if agent_id == self.owner_id || !self.members.contains(&agent_id) {
            return Err(AgentGroupError::MissingMember);
        }
        self.members.retain(|member| *member != agent_id);
        self.member_projects
            .retain(|(member, _)| *member != agent_id);
        self.memberships
            .retain(|membership| membership.agent_id != agent_id);
        self.validate()
    }

    pub fn restore_memberships(
        &mut self,
        snapshot: Vec<AgentGroupMembership>,
    ) -> Result<(), AgentGroupError> {
        if snapshot.is_empty() || snapshot.len() > MAX_GROUP_MEMBERS {
            return Err(AgentGroupError::InvalidMembershipSnapshot);
        }
        self.members = snapshot
            .iter()
            .map(|membership| membership.agent_id)
            .collect();
        self.member_projects = snapshot
            .iter()
            .map(|membership| (membership.agent_id, membership.project_id))
            .collect();
        self.memberships = snapshot;
        self.validate()
    }

    pub fn domain_error(&self) -> Result<(), DomainError> {
        self.validate()
            .map_err(|error| DomainError::Validation(error.to_string()))
    }
}
