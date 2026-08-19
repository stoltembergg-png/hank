//! Erros de domínio e infraestrutura.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Invalid state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Duplicate entity: {0}")]
    Duplicate(String),

    #[error("Invariant violation: {0}")]
    InvariantViolation(String),

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Permission denied: {capability} - {reason}")]
    PermissionDenied { capability: String, reason: String },

    #[error("Budget exceeded: {budget_type} - limit {limit}, used {used}")]
    BudgetExceeded {
        budget_type: String,
        limit: String,
        used: String,
    },

    #[error("Capability not available: {0}")]
    CapabilityUnavailable(String),

    #[error("Provider error: {provider} - {message}")]
    ProviderError { provider: String, message: String },

    #[error("Tool execution failed: {tool} - {message}")]
    ToolError { tool: String, message: String },

    #[error("Workflow error: {workflow} - {message}")]
    WorkflowError { workflow: String, message: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("UUID error: {0}")]
    Uuid(#[from] uuid::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type DomainResult<T> = Result<T, DomainError>;
