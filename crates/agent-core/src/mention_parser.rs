//! Typed, side-effect-free member mention parser.

use crate::{AgentGroupMembership, AgentId, ProjectId};
use std::collections::HashSet;
use std::str::FromStr;
use thiserror::Error;

pub const MAX_MENTION_INPUT_BYTES: usize = 8 * 1024;
pub const MAX_MENTION_TARGETS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MentionTarget {
    pub agent_id: AgentId,
    pub project_id: ProjectId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MentionParseResult {
    pub targets: Vec<MentionTarget>,
    pub invocation_requested: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum MentionError {
    #[error("mention input is too large")]
    InputTooLarge,
    #[error("mention target is unknown or malformed")]
    UnknownMention,
    #[error("mention target is outside the parser project scope")]
    CrossProjectMention,
    #[error("mention target limit is exceeded")]
    TargetLimit,
}

#[derive(Debug, Clone)]
pub struct MentionParser {
    project_id: ProjectId,
    memberships: Vec<AgentGroupMembership>,
}

impl MentionParser {
    pub fn new(project_id: ProjectId, memberships: Vec<AgentGroupMembership>) -> Self {
        Self {
            project_id,
            memberships,
        }
    }

    pub fn parse(&self, input: &str) -> Result<MentionParseResult, MentionError> {
        if input.len() > MAX_MENTION_INPUT_BYTES {
            return Err(MentionError::InputTooLarge);
        }
        let mut targets = Vec::new();
        let mut seen = HashSet::new();
        for token in input
            .split_whitespace()
            .filter(|token| token.starts_with("@agent:"))
        {
            let raw = token
                .trim_start_matches("@agent:")
                .trim_matches(|value: char| matches!(value, ',' | '.' | '!' | '?' | ')' | '('));
            let agent_id = AgentId::from_str(raw).map_err(|_| MentionError::UnknownMention)?;
            let membership = self
                .memberships
                .iter()
                .find(|membership| membership.agent_id == agent_id)
                .ok_or(MentionError::UnknownMention)?;
            if membership.project_id != self.project_id {
                return Err(MentionError::CrossProjectMention);
            }
            if seen.insert(agent_id) {
                if targets.len() >= MAX_MENTION_TARGETS {
                    return Err(MentionError::TargetLimit);
                }
                targets.push(MentionTarget {
                    agent_id,
                    project_id: self.project_id,
                });
            }
        }
        Ok(MentionParseResult {
            targets,
            invocation_requested: false,
        })
    }
}
