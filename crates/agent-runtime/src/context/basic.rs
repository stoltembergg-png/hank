//! Initial deterministic layer assembly over the provider-neutral context contract.

use super::{
    ContextBuildError, ContextBuildResult, ContextBuilder, ContextOmission, ContextOmissionReason,
    ContextRequest, ContextSource, ContextSourceKind,
};
use agent_core::ids::{AgentId, ProjectId};
use provider_core::CancellationToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasicContextLayer {
    Security,
    System,
    Project,
    Agent,
    Conversation,
    Task,
    Tools,
}

impl BasicContextLayer {
    fn expected_kind(self) -> ContextSourceKind {
        match self {
            Self::Security => ContextSourceKind::Security,
            Self::System => ContextSourceKind::System,
            Self::Project => ContextSourceKind::Project,
            Self::Agent | Self::Task => ContextSourceKind::Agent,
            Self::Conversation => ContextSourceKind::User,
            Self::Tools => ContextSourceKind::Tool,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicContextSource {
    pub layer: BasicContextLayer,
    pub source: ContextSource,
}

impl BasicContextSource {
    pub fn new(layer: BasicContextLayer, source: ContextSource) -> Result<Self, ContextBuildError> {
        if source.source_id.trim().is_empty() {
            return Err(ContextBuildError::Invalid);
        }
        Ok(Self { layer, source })
    }
}

#[derive(Debug, Clone)]
pub struct BasicContextRequest {
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub sources: Vec<BasicContextSource>,
    pub max_tokens: u32,
    pub conversation_window: usize,
    pub cancellation: CancellationToken,
}

impl BasicContextRequest {
    pub fn new(
        project_id: ProjectId,
        agent_id: AgentId,
        sources: Vec<BasicContextSource>,
        max_tokens: u32,
        conversation_window: usize,
        cancellation: CancellationToken,
    ) -> Result<Self, ContextBuildError> {
        if conversation_window == 0 {
            return Err(ContextBuildError::Invalid);
        }
        Ok(Self {
            project_id,
            agent_id,
            sources,
            max_tokens,
            conversation_window,
            cancellation,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicContextResult {
    pub context: ContextBuildResult,
}

pub struct BasicContextBuilder;

impl BasicContextBuilder {
    pub fn build(request: BasicContextRequest) -> Result<BasicContextResult, ContextBuildError> {
        if request.cancellation.is_cancelled() {
            return Err(ContextBuildError::Cancelled);
        }
        let mut allowed = Vec::new();
        let mut omissions = Vec::new();
        let conversation_indices: Vec<usize> = request
            .sources
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                (item.layer == BasicContextLayer::Conversation).then_some(index)
            })
            .collect();
        let first_conversation = conversation_indices
            .len()
            .saturating_sub(request.conversation_window);
        let mut seen_conversation = 0;

        for item in request.sources {
            if request.cancellation.is_cancelled() {
                return Err(ContextBuildError::Cancelled);
            }
            if item.source.kind != item.layer.expected_kind() {
                omissions.push(ContextOmission {
                    source_id: item.source.source_id,
                    reason: ContextOmissionReason::Disallowed,
                });
                continue;
            }
            if item.layer == BasicContextLayer::Conversation {
                if seen_conversation < first_conversation {
                    omissions.push(ContextOmission {
                        source_id: item.source.source_id,
                        reason: ContextOmissionReason::ConversationWindow,
                    });
                    seen_conversation += 1;
                    continue;
                }
                seen_conversation += 1;
            }
            allowed.push(item.source);
        }

        let core_request = ContextRequest::new(
            request.project_id,
            request.agent_id,
            allowed,
            request.max_tokens,
            request.cancellation,
        )?;
        let mut context = ContextBuilder::build(core_request)?;
        omissions.append(&mut context.omissions);
        context.omissions = omissions;
        Ok(BasicContextResult { context })
    }
}
