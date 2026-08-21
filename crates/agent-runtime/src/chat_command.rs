//! Thin runtime router for the typed protocol ChatCommand boundary.

use agent_protocol::chat_command::{
    ChatCommand, ChatCommandError, ChatCommandRegistry, ChatCommandStatus,
};
use futures_util::future::BoxFuture;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatCommandHandle {
    pub command_id: String,
    pub session_id: agent_protocol::SessionId,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatDispatchResult {
    pub status: ChatCommandStatus,
    pub handle: Option<ChatCommandHandle>,
}

#[derive(Debug, Error)]
pub enum ChatDispatchError {
    #[error("chat command protocol rejected command: {0}")]
    Protocol(#[from] ChatCommandError),
    #[error("chat command dispatcher rejected command")]
    Dispatcher,
}

pub trait ChatCommandDispatcher: Send + Sync {
    fn dispatch<'a>(
        &'a self,
        command: ChatCommand,
    ) -> BoxFuture<'a, Result<ChatCommandHandle, ChatDispatchError>>;
}

pub struct ChatCommandRouter<D> {
    registry: Arc<ChatCommandRegistry>,
    dispatcher: Arc<D>,
}

impl<D: ChatCommandDispatcher> ChatCommandRouter<D> {
    pub fn new(registry: Arc<ChatCommandRegistry>, dispatcher: Arc<D>) -> Self {
        Self {
            registry,
            dispatcher,
        }
    }

    pub async fn route(
        &self,
        command: ChatCommand,
    ) -> Result<ChatDispatchResult, ChatDispatchError> {
        let status = self.registry.accept(&command)?;
        if status != ChatCommandStatus::Accepted {
            return Ok(ChatDispatchResult {
                status,
                handle: None,
            });
        }
        let handle = self.dispatcher.dispatch(command).await?;
        Ok(ChatDispatchResult {
            status,
            handle: Some(handle),
        })
    }
}
