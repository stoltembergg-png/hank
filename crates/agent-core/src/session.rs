//! Entidades Session e Message de domínio.

use crate::ids::{AgentId, MessageId, ProjectId, SessionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Estado da sessão
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Paused,
    Closed,
    Archived,
}

/// Sessão de chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub status: SessionStatus,
    pub title: Option<String>,
    pub message_count: usize,
    pub token_count: u64,
    pub cost_usd: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

/// Papel da mensagem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
    ToolResult,
}

/// Mensagem de sessão
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub session_id: SessionId,
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub tool_results: Vec<ToolResult>,
    pub tokens: u32,
    pub cost_usd: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
}

impl Session {
    pub fn new(project_id: ProjectId, agent_id: AgentId) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            project_id,
            agent_id,
            status: SessionStatus::Active,
            title: None,
            message_count: 0,
            token_count: 0,
            cost_usd: 0.0,
            created_at: now,
            updated_at: now,
            closed_at: None,
        }
    }

    pub fn close(&mut self) {
        self.status = SessionStatus::Closed;
        self.closed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn add_message(&mut self, message: Message) {
        self.message_count += 1;
        self.token_count += message.tokens as u64;
        self.cost_usd += message.cost_usd;
        self.updated_at = Utc::now();
    }
}
