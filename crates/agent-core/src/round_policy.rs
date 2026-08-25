//! Bounded group round state machine; no scheduler or model override.

use crate::{AgentId, ProjectId};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RoundStopReason {
    #[error("maximum rounds reached")]
    MaxRounds,
    #[error("maximum turns reached")]
    MaxTurns,
    #[error("no progress detected")]
    NoProgress,
    #[error("budget exceeded")]
    BudgetExceeded,
    #[error("round cancelled")]
    Cancelled,
    #[error("duplicate turn")]
    DuplicateTurn,
}

#[derive(Debug)]
pub struct RoundPolicy {
    project_id: ProjectId,
    group_id: uuid::Uuid,
    session_id: uuid::Uuid,
    moderator_id: AgentId,
    max_rounds: u32,
    max_turns: u32,
    current_round: u32,
    current_turns: u32,
    no_progress: u32,
    turns: HashSet<uuid::Uuid>,
    terminal: Option<RoundStopReason>,
}

impl RoundPolicy {
    pub fn new(
        project_id: ProjectId,
        group_id: uuid::Uuid,
        session_id: uuid::Uuid,
        max_rounds: u32,
        max_turns: u32,
        moderator_id: AgentId,
    ) -> Result<Self, RoundStopReason> {
        if max_rounds == 0 || max_turns == 0 {
            return Err(RoundStopReason::MaxRounds);
        }
        Ok(Self {
            project_id,
            group_id,
            session_id,
            moderator_id,
            max_rounds,
            max_turns,
            current_round: 0,
            current_turns: 0,
            no_progress: 0,
            turns: HashSet::new(),
            terminal: None,
        })
    }
    pub fn begin_round(&mut self) -> Result<u32, RoundStopReason> {
        if let Some(reason) = self.terminal {
            return Err(reason);
        }
        if self.current_round >= self.max_rounds {
            self.terminal = Some(RoundStopReason::MaxRounds);
            return Err(RoundStopReason::MaxRounds);
        }
        self.current_round += 1;
        self.no_progress = 0;
        Ok(self.current_round)
    }
    pub fn record_turn(
        &mut self,
        turn_id: uuid::Uuid,
        progress: bool,
    ) -> Result<(), RoundStopReason> {
        if let Some(reason) = self.terminal {
            return Err(reason);
        }
        if !self.turns.insert(turn_id) {
            return Err(RoundStopReason::DuplicateTurn);
        }
        if self.current_turns >= self.max_turns {
            self.terminal = Some(RoundStopReason::MaxTurns);
            return Err(RoundStopReason::MaxTurns);
        }
        self.current_turns += 1;
        if progress {
            self.no_progress = 0;
        } else {
            self.no_progress += 1;
        }
        if self.no_progress >= 2 {
            self.terminal = Some(RoundStopReason::NoProgress);
            return Err(RoundStopReason::NoProgress);
        }
        Ok(())
    }
    pub fn stop(&mut self, reason: RoundStopReason) {
        self.terminal = Some(reason);
    }
    pub fn is_terminal(&self) -> bool {
        self.terminal.is_some()
    }
    pub fn current_turns(&self) -> u32 {
        self.current_turns
    }
    pub fn project_id(&self) -> ProjectId {
        self.project_id
    }
    pub fn group_id(&self) -> uuid::Uuid {
        self.group_id
    }
    pub fn session_id(&self) -> uuid::Uuid {
        self.session_id
    }
    pub fn moderator_id(&self) -> AgentId {
        self.moderator_id
    }
}
