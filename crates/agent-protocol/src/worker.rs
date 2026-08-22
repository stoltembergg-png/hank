//! Contrato versionado e mínimo entre o Agent Runtime e um worker Python
//! opcional.
//!
//! O contrato define mensagens, identidade, ciclo de vida e validação
//! fail-closed. Ele é independente de qualquer implementação Python: o core
//! compila, testa e opera sem Python. Payloads são dados bounded — erros e
//! cancelamentos nunca carregam instrução executável nem segredo.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::capability::Capability;
use crate::envelope::TerminalResult;
use crate::ids::{ProjectId, RequestId, SessionId, TaskId, TraceId};

pub const WORKER_PROTOCOL_SCHEMA_VERSION: u32 = 1;
pub const WORKER_PROTOCOL_NAME: &str = "hank://worker/protocol";

const MAX_ID_LEN: usize = 128;
const MAX_PAYLOAD_BYTES: usize = 65_536;
const MAX_PENDING_REQUESTS: usize = 256;
const MAX_CAPABILITIES: usize = 32;
const MAX_ERROR_DETAIL_LEN: usize = 256;

/// Contexto de identidade transportado por request e response.
///
/// Projeto, sessão, tarefa opcional e trace vinculam cada mensagem ao
/// contexto exato que a originou; responses devem devolver o mesmo contexto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerContext {
    pub project_id: ProjectId,
    pub session_id: SessionId,
    pub task_id: Option<TaskId>,
    pub trace_id: TraceId,
}

/// Código de erro de protocolo; detalhe é transportado à parte e bounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerErrorCode {
    InvalidMessage,
    UnsupportedVersion,
    UnknownRequest,
    InvalidState,
    CapacityFull,
    InternalError,
}

/// Detalhe de erro bounded, sem segredo e sem instrução executável.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerErrorDetail {
    pub code: WorkerErrorCode,
    pub detail: String,
}

/// Motivos bounded de cancelamento de request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerCancelReason {
    User,
    Deadline,
    SessionClosed,
    Shutdown,
}

/// Estado de saúde reportado pelo worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerHealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Motivos bounded de encerramento do worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerShutdownReason {
    User,
    Restart,
    Timeout,
    Failure,
}

/// Mensagem do protocolo worker; serialização determinística via serde.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkerMessage {
    Handshake {
        schema_version: u32,
        worker_id: String,
        protocol_version: u32,
        capabilities: Vec<Capability>,
    },
    HandshakeAccepted {
        schema_version: u32,
        worker_id: String,
        protocol_version: u32,
    },
    Request {
        schema_version: u32,
        request_id: RequestId,
        context: WorkerContext,
        capability: Capability,
        payload: serde_json::Value,
    },
    Response {
        schema_version: u32,
        request_id: RequestId,
        context: WorkerContext,
        result: TerminalResult,
        value: Option<serde_json::Value>,
        error: Option<WorkerErrorDetail>,
    },
    Cancel {
        schema_version: u32,
        request_id: RequestId,
        reason: WorkerCancelReason,
    },
    Health {
        schema_version: u32,
    },
    HealthReport {
        schema_version: u32,
        worker_id: String,
        status: WorkerHealthStatus,
    },
    Error {
        schema_version: u32,
        code: WorkerErrorCode,
        detail: String,
    },
    Shutdown {
        schema_version: u32,
        reason: WorkerShutdownReason,
    },
    ShutdownAck {
        schema_version: u32,
    },
}

impl WorkerMessage {
    /// Validação fail-closed da mensagem isolada: versão, identidade,
    /// bounds e coerência resultado/valor/erro.
    pub fn validate(&self) -> Result<(), WorkerProtocolError> {
        let schema_version = match self {
            Self::Handshake { schema_version, .. }
            | Self::HandshakeAccepted { schema_version, .. }
            | Self::Request { schema_version, .. }
            | Self::Response { schema_version, .. }
            | Self::Cancel { schema_version, .. }
            | Self::Health { schema_version }
            | Self::HealthReport { schema_version, .. }
            | Self::Error { schema_version, .. }
            | Self::Shutdown { schema_version, .. }
            | Self::ShutdownAck { schema_version } => *schema_version,
        };
        if schema_version != WORKER_PROTOCOL_SCHEMA_VERSION {
            return Err(WorkerProtocolError::UnsupportedVersion);
        }
        match self {
            Self::Handshake {
                worker_id,
                protocol_version,
                capabilities,
                ..
            } => {
                if !valid_id(worker_id) {
                    return Err(WorkerProtocolError::InvalidIdentity);
                }
                if *protocol_version != WORKER_PROTOCOL_SCHEMA_VERSION {
                    return Err(WorkerProtocolError::UnsupportedVersion);
                }
                if capabilities.is_empty() || capabilities.len() > MAX_CAPABILITIES {
                    return Err(WorkerProtocolError::InvalidPayload);
                }
            }
            Self::HandshakeAccepted {
                worker_id,
                protocol_version,
                ..
            } => {
                if !valid_id(worker_id) {
                    return Err(WorkerProtocolError::InvalidIdentity);
                }
                if *protocol_version != WORKER_PROTOCOL_SCHEMA_VERSION {
                    return Err(WorkerProtocolError::UnsupportedVersion);
                }
            }
            Self::Request { payload, .. } => {
                let bytes = serde_json::to_vec(payload)
                    .map_err(|_| WorkerProtocolError::InvalidPayload)?
                    .len();
                if bytes == 0 || bytes > MAX_PAYLOAD_BYTES {
                    return Err(WorkerProtocolError::OversizedPayload);
                }
                if payload.is_null() {
                    return Err(WorkerProtocolError::InvalidPayload);
                }
            }
            Self::Response {
                result,
                value,
                error,
                ..
            } => match result {
                TerminalResult::Succeeded => {
                    if error.is_some() {
                        return Err(WorkerProtocolError::InvalidPayload);
                    }
                    if let Some(value) = value {
                        let bytes = serde_json::to_vec(value)
                            .map_err(|_| WorkerProtocolError::InvalidPayload)?
                            .len();
                        if bytes > MAX_PAYLOAD_BYTES {
                            return Err(WorkerProtocolError::OversizedPayload);
                        }
                    }
                }
                TerminalResult::Cancelled | TerminalResult::TimedOut => {
                    if value.is_some() || error.is_some() {
                        return Err(WorkerProtocolError::InvalidPayload);
                    }
                }
                TerminalResult::Rejected
                | TerminalResult::Failed
                | TerminalResult::NotSupported
                | TerminalResult::Blocked => {
                    if value.is_some() {
                        return Err(WorkerProtocolError::InvalidPayload);
                    }
                    let detail = error.as_ref().ok_or(WorkerProtocolError::InvalidPayload)?;
                    validate_error_detail(&detail.detail)?;
                }
            },
            Self::HealthReport { worker_id, .. } => {
                if !valid_id(worker_id) {
                    return Err(WorkerProtocolError::InvalidIdentity);
                }
            }
            Self::Error { detail, .. } => {
                validate_error_detail(detail)?;
            }
            Self::Cancel { .. }
            | Self::Health { .. }
            | Self::Shutdown { .. }
            | Self::ShutdownAck { .. } => {}
        }
        Ok(())
    }
}

/// Erro de protocolo com mensagens fixas e redigidas; nunca carrega payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WorkerProtocolError {
    #[error("worker protocol version is not supported")]
    UnsupportedVersion,
    #[error("worker identity is invalid")]
    InvalidIdentity,
    #[error("worker message payload is invalid")]
    InvalidPayload,
    #[error("worker message payload exceeds the bounded size")]
    OversizedPayload,
    #[error("worker message arrived before handshake completed")]
    NotHandshaked,
    #[error("worker handshake already completed")]
    AlreadyHandshaked,
    #[error("worker message violates the protocol state machine")]
    InvalidState,
    #[error("worker message references an unknown request")]
    UnknownRequest,
    #[error("worker request id was already registered")]
    DuplicateRequest,
    #[error("worker response context differs from the request context")]
    ContextMismatch,
    #[error("worker pending request capacity is full")]
    Backpressure,
    #[error("worker session is shutdown")]
    AfterShutdown,
}

impl fmt::Display for WorkerHealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
        };
        write!(f, "{name}")
    }
}

/// Estado do ciclo de vida da sessão worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerSessionState {
    AwaitingHandshake,
    Handshaking,
    Ready,
    ShuttingDown,
    Shutdown,
}

/// Validador fail-closed da conversa runtime ↔ worker.
///
/// Consome mensagens na ordem observada no canal e rejeita qualquer
/// violação de ciclo de vida, correlação, isolamento de contexto ou limite
/// de capacidade.
#[derive(Debug)]
pub struct WorkerSession {
    state: WorkerSessionState,
    pending: BTreeMap<Uuid, WorkerContext>,
}

impl WorkerSession {
    pub fn new() -> Self {
        Self {
            state: WorkerSessionState::AwaitingHandshake,
            pending: BTreeMap::new(),
        }
    }

    pub fn state(&self) -> WorkerSessionState {
        self.state
    }

    pub fn is_shutdown(&self) -> bool {
        self.state == WorkerSessionState::Shutdown
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn accept(&mut self, message: WorkerMessage) -> Result<(), WorkerProtocolError> {
        message.validate()?;
        if self.state == WorkerSessionState::Shutdown {
            return Err(WorkerProtocolError::AfterShutdown);
        }
        match message {
            WorkerMessage::Handshake { .. } => {
                if self.state != WorkerSessionState::AwaitingHandshake {
                    return Err(WorkerProtocolError::AlreadyHandshaked);
                }
                self.state = WorkerSessionState::Handshaking;
            }
            WorkerMessage::HandshakeAccepted { .. } => {
                if self.state != WorkerSessionState::Handshaking {
                    return Err(WorkerProtocolError::NotHandshaked);
                }
                self.state = WorkerSessionState::Ready;
            }
            WorkerMessage::Request {
                request_id,
                context,
                ..
            } => {
                self.require_ready()?;
                if self.pending.contains_key(&request_id.as_uuid()) {
                    return Err(WorkerProtocolError::DuplicateRequest);
                }
                if self.pending.len() >= MAX_PENDING_REQUESTS {
                    return Err(WorkerProtocolError::Backpressure);
                }
                self.pending.insert(request_id.as_uuid(), context);
            }
            WorkerMessage::Response {
                request_id,
                context,
                ..
            } => {
                self.require_ready()?;
                let expected = *self
                    .pending
                    .get(&request_id.as_uuid())
                    .ok_or(WorkerProtocolError::UnknownRequest)?;
                if expected != context {
                    return Err(WorkerProtocolError::ContextMismatch);
                }
                self.pending.remove(&request_id.as_uuid());
            }
            WorkerMessage::Cancel { request_id, .. } => {
                self.require_ready()?;
                self.pending
                    .remove(&request_id.as_uuid())
                    .ok_or(WorkerProtocolError::UnknownRequest)?;
            }
            WorkerMessage::Health { .. } | WorkerMessage::HealthReport { .. } => {
                self.require_ready()?;
            }
            WorkerMessage::Error { .. } => {
                self.require_ready()?;
            }
            WorkerMessage::Shutdown { .. } => {
                self.require_ready()?;
                self.state = WorkerSessionState::ShuttingDown;
            }
            WorkerMessage::ShutdownAck { .. } => {
                if self.state != WorkerSessionState::ShuttingDown {
                    return Err(WorkerProtocolError::InvalidState);
                }
                self.state = WorkerSessionState::Shutdown;
            }
        }
        Ok(())
    }

    fn require_ready(&self) -> Result<(), WorkerProtocolError> {
        match self.state {
            WorkerSessionState::Ready => Ok(()),
            WorkerSessionState::AwaitingHandshake | WorkerSessionState::Handshaking => {
                Err(WorkerProtocolError::NotHandshaked)
            }
            WorkerSessionState::ShuttingDown | WorkerSessionState::Shutdown => {
                Err(WorkerProtocolError::AfterShutdown)
            }
        }
    }
}

impl Default for WorkerSession {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_error_detail(detail: &str) -> Result<(), WorkerProtocolError> {
    if detail.is_empty() || detail.len() > MAX_ERROR_DETAIL_LEN {
        return Err(WorkerProtocolError::InvalidPayload);
    }
    if detail.chars().any(|c| c.is_control()) {
        return Err(WorkerProtocolError::InvalidPayload);
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_LEN && !value.chars().any(char::is_control)
}
