//! Versioned typed chat command envelope and bounded deduplication registry.

use crate::ids::{AgentId, ProjectId, SessionId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::{Arc, Mutex};
use thiserror::Error;

const MAX_ID_LEN: usize = 128;
const MAX_TEXT_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallerIdentity {
    pub caller_id: String,
    pub class: String,
}

impl CallerIdentity {
    pub fn new(
        caller_id: impl Into<String>,
        class: impl Into<String>,
    ) -> Result<Self, ChatCommandError> {
        let caller_id = caller_id.into();
        let class = class.into();
        if !valid_id(&caller_id) || !valid_id(&class) {
            return Err(ChatCommandError::Invalid);
        }
        Ok(Self { caller_id, class })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCommand {
    pub schema_version: u32,
    pub command_id: String,
    pub caller: CallerIdentity,
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub session_id: SessionId,
    pub text: String,
    pub generation: u64,
    pub cancellation_id: String,
}

impl ChatCommand {
    pub const SCHEMA_VERSION: u32 = 1;

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        command_id: impl Into<String>,
        caller: CallerIdentity,
        project_id: ProjectId,
        agent_id: AgentId,
        session_id: SessionId,
        text: impl Into<String>,
        generation: u64,
        cancellation_id: impl Into<String>,
    ) -> Result<Self, ChatCommandError> {
        let command_id = command_id.into();
        let text = text.into();
        let cancellation_id = cancellation_id.into();
        if !valid_id(&command_id)
            || !valid_id(&cancellation_id)
            || text.is_empty()
            || text.len() > MAX_TEXT_BYTES
            || text.chars().any(char::is_control)
            || generation == 0
            || contains_forbidden_marker(&text)
        {
            return Err(ChatCommandError::Invalid);
        }
        Ok(Self {
            schema_version: Self::SCHEMA_VERSION,
            command_id,
            caller,
            project_id,
            agent_id,
            session_id,
            text,
            generation,
            cancellation_id,
        })
    }

    pub fn status(&self) -> ChatCommandStatus {
        ChatCommandStatus::Accepted
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatCommandStatus {
    Accepted,
    Duplicate,
    Stale,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ChatCommandError {
    #[error("chat command is invalid")]
    Invalid,
    #[error("chat command registry capacity reached")]
    Capacity,
    #[error("chat command registry lock unavailable")]
    Lock,
}

pub struct ChatCommandRegistry {
    max: usize,
    commands: Arc<Mutex<BTreeSet<String>>>,
    latest_generation: Arc<Mutex<BTreeMap<String, u64>>>,
}

impl fmt::Debug for ChatCommandRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ChatCommandRegistry")
            .field("max", &self.max)
            .field(
                "command_count",
                &self.commands.lock().map(|set| set.len()).unwrap_or(0),
            )
            .finish()
    }
}

impl ChatCommandRegistry {
    pub fn new(max: usize) -> Result<Self, ChatCommandError> {
        if max == 0 {
            return Err(ChatCommandError::Capacity);
        }
        Ok(Self {
            max,
            commands: Arc::new(Mutex::new(BTreeSet::new())),
            latest_generation: Arc::new(Mutex::new(BTreeMap::new())),
        })
    }

    pub fn accept(&self, command: &ChatCommand) -> Result<ChatCommandStatus, ChatCommandError> {
        if command.schema_version != ChatCommand::SCHEMA_VERSION {
            return Err(ChatCommandError::Invalid);
        }
        let mut commands = self.commands.lock().map_err(|_| ChatCommandError::Lock)?;
        if commands.contains(&command.command_id) {
            return Ok(ChatCommandStatus::Duplicate);
        }
        let mut generations = self
            .latest_generation
            .lock()
            .map_err(|_| ChatCommandError::Lock)?;
        if generations
            .get(&command.session_id.to_string())
            .is_some_and(|latest| command.generation < *latest)
        {
            return Ok(ChatCommandStatus::Stale);
        }
        if commands.len() >= self.max {
            return Err(ChatCommandError::Capacity);
        }
        commands.insert(command.command_id.clone());
        generations
            .entry(command.session_id.to_string())
            .and_modify(|latest| *latest = (*latest).max(command.generation))
            .or_insert(command.generation);
        Ok(ChatCommandStatus::Accepted)
    }
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_ID_LEN
        && value.chars().all(|character| !character.is_control())
}

fn contains_forbidden_marker(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    [
        "api_key",
        "authorization:",
        "password",
        "secret",
        "token",
        "bearer",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}
