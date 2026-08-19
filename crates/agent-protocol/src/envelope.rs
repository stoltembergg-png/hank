//! Envelopes de comando, resultado e evento para comunicação entre camadas.
//!
//! Define a estrutura padrão de todas as operações que cruzam fronteiras
//! arquiteturais, garantindo rastreabilidade, idempotência e validação.

use crate::capability::Capability;
use crate::ids::{AgentId, ProjectId, RequestId, TraceId};
use crate::policy::PolicyDecision;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Resultado terminal de uma operação
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalResult {
    Succeeded,
    Rejected,
    Failed,
    Cancelled,
    TimedOut,
    NotSupported,
    Blocked,
}

/// Envelope base que toda operação deve carregar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseEnvelope {
    pub schema_version: u32,
    pub request_id: RequestId,
    pub trace_id: TraceId,
    pub project_id: ProjectId,
    pub actor_id: Option<AgentId>,
    pub capability: Capability,
    pub deadline: Option<DateTime<Utc>>,
    pub idempotency_key: Option<String>,
}

/// Comando enviado para execução
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEnvelope<P> {
    #[serde(flatten)]
    pub base: BaseEnvelope,
    pub payload: P,
}

/// Resultado de um comando
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultEnvelope<R, E> {
    #[serde(flatten)]
    pub base: BaseEnvelope,
    pub result: TerminalResult,
    pub value: Option<R>,
    pub error: Option<E>,
    pub policy_decision: Option<PolicyDecision>,
    pub redaction_digest: Option<String>,
}

/// Evento assíncrono (notificação, progresso, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope<E> {
    #[serde(flatten)]
    pub base: BaseEnvelope,
    pub event_type: String,
    pub payload: E,
    pub sequence: u64,
}

impl<P> CommandEnvelope<P> {
    pub fn new(project_id: ProjectId, capability: Capability, payload: P) -> Self {
        Self {
            base: BaseEnvelope {
                schema_version: 1,
                request_id: RequestId::new(),
                trace_id: TraceId::new(),
                project_id,
                actor_id: None,
                capability,
                deadline: None,
                idempotency_key: None,
            },
            payload,
        }
    }
}

impl<R, E> ResultEnvelope<R, E> {
    pub fn success(base: BaseEnvelope, value: R) -> Self {
        Self {
            base,
            result: TerminalResult::Succeeded,
            value: Some(value),
            error: None,
            policy_decision: None,
            redaction_digest: None,
        }
    }

    pub fn rejected(base: BaseEnvelope, error: E, policy_decision: PolicyDecision) -> Self {
        Self {
            base,
            result: TerminalResult::Rejected,
            value: None,
            error: Some(error),
            policy_decision: Some(policy_decision),
            redaction_digest: None,
        }
    }
}
