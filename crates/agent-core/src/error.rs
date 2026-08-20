//! Erros de domínio e envelope seguro para fronteiras externas.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainErrorCode {
    InvalidStateTransition,
    NotFound,
    Duplicate,
    InvariantViolation,
    Validation,
    PermissionDenied,
    BudgetExceeded,
    CapabilityUnavailable,
    Provider,
    Tool,
    Workflow,
    Serialization,
    Uuid,
    Io,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Retryability {
    Never,
    Conditional,
    Safe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainErrorEnvelope {
    pub code: DomainErrorCode,
    pub retryability: Retryability,
    pub message: String,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Invalid state transition")]
    InvalidStateTransition { from: String, to: String },
    #[error("Entity not found")]
    NotFound(String),
    #[error("Duplicate entity")]
    Duplicate(String),
    #[error("Invariant violation")]
    InvariantViolation(String),
    #[error("Validation failed")]
    Validation(String),
    #[error("Permission denied")]
    PermissionDenied { capability: String, reason: String },
    #[error("Budget exceeded")]
    BudgetExceeded {
        budget_type: String,
        limit: String,
        used: String,
    },
    #[error("Capability not available")]
    CapabilityUnavailable(String),
    #[error("Provider error")]
    ProviderError { provider: String, message: String },
    #[error("Tool execution failed")]
    ToolError { tool: String, message: String },
    #[error("Workflow error")]
    WorkflowError { workflow: String, message: String },
    #[error("Serialization error")]
    Serialization(#[from] serde_json::Error),
    #[error("UUID error")]
    Uuid(#[from] uuid::Error),
    #[error("IO error")]
    Io(#[from] std::io::Error),
    #[error("Concurrency conflict")]
    ConcurrencyConflict { expected: String, actual: String },
}

impl DomainError {
    pub fn code(&self) -> DomainErrorCode {
        match self {
            Self::InvalidStateTransition { .. } => DomainErrorCode::InvalidStateTransition,
            Self::NotFound(_) => DomainErrorCode::NotFound,
            Self::Duplicate(_) => DomainErrorCode::Duplicate,
            Self::InvariantViolation(_) => DomainErrorCode::InvariantViolation,
            Self::Validation(_) => DomainErrorCode::Validation,
            Self::PermissionDenied { .. } => DomainErrorCode::PermissionDenied,
            Self::BudgetExceeded { .. } => DomainErrorCode::BudgetExceeded,
            Self::CapabilityUnavailable(_) => DomainErrorCode::CapabilityUnavailable,
            Self::ProviderError { .. } => DomainErrorCode::Provider,
            Self::ToolError { .. } => DomainErrorCode::Tool,
            Self::WorkflowError { .. } => DomainErrorCode::Workflow,
            Self::Serialization(_) => DomainErrorCode::Serialization,
            Self::Uuid(_) => DomainErrorCode::Uuid,
            Self::Io(_) => DomainErrorCode::Io,
            Self::ConcurrencyConflict { .. } => DomainErrorCode::InvariantViolation,
        }
    }

    pub fn retryability(&self) -> Retryability {
        match self {
            Self::ProviderError { .. } | Self::ToolError { .. } | Self::WorkflowError { .. } => {
                Retryability::Conditional
            }
            Self::Io(_) => Retryability::Safe,
            Self::ConcurrencyConflict { .. } => Retryability::Never,
            _ => Retryability::Never,
        }
    }

    pub fn envelope(&self, correlation_id: Option<String>) -> DomainErrorEnvelope {
        DomainErrorEnvelope {
            code: self.code(),
            retryability: self.retryability(),
            message: self.to_string(),
            correlation_id,
        }
    }
}

pub type DomainResult<T> = Result<T, DomainError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_has_stable_code_retryability_and_correlation() {
        let error = DomainError::ProviderError {
            provider: "synthetic".into(),
            message: "secret-token-redacted".into(),
        };
        let envelope = error.envelope(Some("corr-1".into()));
        assert_eq!(envelope.code, DomainErrorCode::Provider);
        assert_eq!(envelope.retryability, Retryability::Conditional);
        assert_eq!(envelope.correlation_id.as_deref(), Some("corr-1"));
        assert_eq!(envelope.message, "Provider error");
        assert!(!envelope.message.contains("secret-token"));
    }

    #[test]
    fn validation_and_permission_errors_are_not_retryable() {
        assert_eq!(
            DomainError::Validation("bad input".into()).retryability(),
            Retryability::Never
        );
        assert_eq!(
            DomainError::PermissionDenied {
                capability: "read".into(),
                reason: "denied".into()
            }
            .code(),
            DomainErrorCode::PermissionDenied
        );
    }
}
