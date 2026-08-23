//! Provider-neutral context builder interface and deterministic bounded selector.

use agent_core::ids::{AgentId, ProjectId};
use provider_core::CancellationToken;
use std::collections::BTreeSet;
use thiserror::Error;

const MAX_SOURCES: usize = 128;
const MAX_SOURCE_ID_LEN: usize = 128;
const MAX_SOURCE_BYTES: usize = 1_048_576;
const MAX_TOTAL_TOKENS: u32 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContextSourceKind {
    Security,
    System,
    Project,
    Agent,
    User,
    Provider,
    Tool,
    Memory,
    Skill,
}

impl ContextSourceKind {
    fn priority(self) -> u8 {
        match self {
            Self::Security => 0,
            Self::System => 1,
            Self::Project => 2,
            Self::Agent => 3,
            Self::User => 4,
            Self::Provider => 5,
            Self::Tool => 6,
            Self::Memory => 7,
            Self::Skill => 8,
        }
    }

    fn untrusted(self) -> bool {
        matches!(
            self,
            Self::User | Self::Provider | Self::Tool | Self::Memory | Self::Skill
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextSource {
    pub source_id: String,
    pub kind: ContextSourceKind,
    pub content: String,
    pub estimated_tokens: u32,
    pub duplicate_key: Option<String>,
    pub sensitive: bool,
}

impl ContextSource {
    pub fn new(
        source_id: impl Into<String>,
        kind: ContextSourceKind,
        content: impl Into<String>,
        estimated_tokens: u32,
    ) -> Result<Self, ContextBuildError> {
        let source_id = source_id.into();
        let content = content.into();
        if source_id.trim().is_empty()
            || source_id.len() > MAX_SOURCE_ID_LEN
            || source_id.chars().any(char::is_control)
            || content.len() > MAX_SOURCE_BYTES
            || content.chars().any(char::is_control)
            || estimated_tokens == 0
            || estimated_tokens > MAX_TOTAL_TOKENS
            || contains_forbidden_marker(&content)
        {
            return Err(ContextBuildError::Invalid);
        }
        Ok(Self {
            source_id,
            kind,
            content,
            estimated_tokens,
            duplicate_key: None,
            sensitive: false,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ContextRequest {
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub sources: Vec<ContextSource>,
    pub required_source_ids: Vec<String>,
    pub max_tokens: u32,
    pub cancellation: CancellationToken,
}

impl ContextRequest {
    pub fn new(
        project_id: ProjectId,
        agent_id: AgentId,
        sources: Vec<ContextSource>,
        max_tokens: u32,
        cancellation: CancellationToken,
    ) -> Result<Self, ContextBuildError> {
        if sources.len() > MAX_SOURCES || !(1..=MAX_TOTAL_TOKENS).contains(&max_tokens) {
            return Err(ContextBuildError::Invalid);
        }
        Ok(Self {
            project_id,
            agent_id,
            sources,
            required_source_ids: Vec::new(),
            max_tokens,
            cancellation,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextOmissionReason {
    Budget,
    Duplicate,
    Sensitive,
    Missing,
    ConversationWindow,
    Disallowed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextOmission {
    pub source_id: String,
    pub reason: ContextOmissionReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextEntry {
    pub source_id: String,
    pub kind: ContextSourceKind,
    pub content: String,
    pub untrusted: bool,
    pub tool_executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextBuildResult {
    pub entries: Vec<ContextEntry>,
    pub omissions: Vec<ContextOmission>,
    pub consumed_tokens: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ContextBuildError {
    #[error("context request or source is invalid")]
    Invalid,
    #[error("context build was cancelled")]
    Cancelled,
}

pub struct ContextBuilder;

impl ContextBuilder {
    pub fn build(mut request: ContextRequest) -> Result<ContextBuildResult, ContextBuildError> {
        if request.cancellation.is_cancelled() {
            return Err(ContextBuildError::Cancelled);
        }
        request.sources.sort_by(|left, right| {
            left.kind
                .priority()
                .cmp(&right.kind.priority())
                .then_with(|| left.source_id.cmp(&right.source_id))
        });
        let mut entries = Vec::new();
        let mut omissions = Vec::new();
        let mut seen = BTreeSet::new();
        let mut consumed_tokens: u32 = 0;
        for source in request.sources {
            if request.cancellation.is_cancelled() {
                return Err(ContextBuildError::Cancelled);
            }
            if let Some(key) = &source.duplicate_key {
                if !seen.insert(key.clone()) {
                    omissions.push(ContextOmission {
                        source_id: source.source_id,
                        reason: ContextOmissionReason::Duplicate,
                    });
                    continue;
                }
            }
            if source.sensitive {
                omissions.push(ContextOmission {
                    source_id: source.source_id,
                    reason: ContextOmissionReason::Sensitive,
                });
                continue;
            }
            if consumed_tokens.saturating_add(source.estimated_tokens) > request.max_tokens {
                omissions.push(ContextOmission {
                    source_id: source.source_id,
                    reason: ContextOmissionReason::Budget,
                });
                continue;
            }
            consumed_tokens = consumed_tokens.saturating_add(source.estimated_tokens);
            entries.push(ContextEntry {
                source_id: source.source_id,
                kind: source.kind,
                content: source.content,
                untrusted: source.kind.untrusted(),
                tool_executable: false,
            });
        }
        let included: BTreeSet<String> = entries
            .iter()
            .map(|entry| entry.source_id.clone())
            .collect();
        let omitted: BTreeSet<String> = omissions
            .iter()
            .map(|omission| omission.source_id.clone())
            .collect();
        for required in request.required_source_ids {
            if !included.contains(required.as_str()) && !omitted.contains(required.as_str()) {
                omissions.push(ContextOmission {
                    source_id: required,
                    reason: ContextOmissionReason::Missing,
                });
            }
        }
        Ok(ContextBuildResult {
            entries,
            omissions,
            consumed_tokens,
        })
    }
}

pub mod basic;
pub mod memory_selector;

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
