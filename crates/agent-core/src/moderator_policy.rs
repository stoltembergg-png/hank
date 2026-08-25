//! Explicit, versioned moderator routing policy.

use crate::{AgentId, ProjectId};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModeratorDecision {
    Route { target: AgentId },
    DenyTargetNotEligible,
    DenyCycleOrDepth,
    DenyBudget,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PolicyRollbackError {
    #[error("policy snapshot is not older than current policy")]
    InvalidSnapshot,
    #[error("policy participant limit is invalid")]
    InvalidLimit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeratorPolicySnapshot {
    pub version: u64,
    pub max_participants: usize,
    pub eligible: HashSet<AgentId>,
}

#[derive(Debug, Clone)]
pub struct GroupModeratorPolicy {
    project_id: ProjectId,
    group_id: uuid::Uuid,
    moderator_id: AgentId,
    version: u64,
    max_participants: usize,
    eligible: HashSet<AgentId>,
}

impl GroupModeratorPolicy {
    pub fn new(
        project_id: ProjectId,
        group_id: uuid::Uuid,
        moderator_id: AgentId,
        max_participants: usize,
    ) -> Result<Self, PolicyRollbackError> {
        if max_participants == 0 {
            return Err(PolicyRollbackError::InvalidLimit);
        }
        Ok(Self {
            project_id,
            group_id,
            moderator_id,
            version: 1,
            max_participants,
            eligible: HashSet::new(),
        })
    }
    pub fn set_max_participants(&mut self, value: usize) -> Result<(), PolicyRollbackError> {
        if value == 0 || value < self.eligible.len() {
            return Err(PolicyRollbackError::InvalidLimit);
        }
        self.max_participants = value;
        self.version += 1;
        Ok(())
    }

    pub fn add_eligible_member(&mut self, agent_id: AgentId) -> Result<(), PolicyRollbackError> {
        if self.eligible.len() >= self.max_participants && !self.eligible.contains(&agent_id) {
            return Err(PolicyRollbackError::InvalidLimit);
        }
        self.eligible.insert(agent_id);
        self.version += 1;
        Ok(())
    }
    pub fn decide(
        &self,
        target: AgentId,
        cycle_pass: bool,
        depth_pass: bool,
        budget_pass: bool,
    ) -> ModeratorDecision {
        if !self.eligible.contains(&target) {
            return ModeratorDecision::DenyTargetNotEligible;
        }
        if !cycle_pass || !depth_pass {
            return ModeratorDecision::DenyCycleOrDepth;
        }
        if !budget_pass {
            return ModeratorDecision::DenyBudget;
        }
        ModeratorDecision::Route { target }
    }
    pub fn snapshot(&self) -> ModeratorPolicySnapshot {
        ModeratorPolicySnapshot {
            version: self.version,
            max_participants: self.max_participants,
            eligible: self.eligible.clone(),
        }
    }
    pub fn rollback(
        &mut self,
        snapshot: ModeratorPolicySnapshot,
    ) -> Result<(), PolicyRollbackError> {
        if snapshot.version >= self.version || snapshot.max_participants == 0 {
            return Err(PolicyRollbackError::InvalidSnapshot);
        }
        self.max_participants = snapshot.max_participants;
        self.eligible = snapshot.eligible;
        self.version += 1;
        Ok(())
    }
    pub fn version(&self) -> u64 {
        self.version
    }
    pub fn moderator_id(&self) -> AgentId {
        self.moderator_id
    }
    pub fn project_id(&self) -> ProjectId {
        self.project_id
    }
    pub fn group_id(&self) -> uuid::Uuid {
        self.group_id
    }
}
