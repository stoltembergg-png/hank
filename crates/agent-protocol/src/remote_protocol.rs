//! Protocolo remoto versionado: handshake, catálogo de comandos/eventos,
//! correlação de requests, identidade de peer/node/project e modelo de erro.
//!
//! Este módulo define os contratos de protocolo que operam sobre o transporte
//! runtime-neutral ([`crate::runtime_transport::RuntimeTransport`]).
//! Autenticação, WebSocket, dispatch remoto e isolamento de credenciais
//! pertencem a cards posteriores (PR-246+).

use crate::ids::ProjectId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Tamanho máximo de payload serializado em bytes.
pub const MAX_PAYLOAD: usize = 64 * 1024;

/// Tamanho máximo de identidade textual (peer, node).
pub const MAX_ID_LEN: usize = 128;

// ---------------------------------------------------------------------------
// Protocol revision
// ---------------------------------------------------------------------------

/// Par major/minor para negociação de versão de protocolo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProtocolRevision {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolRevision {
    /// Versão 1.0 do protocolo remoto.
    pub const V1_0: Self = Self { major: 1, minor: 0 };

    /// `true` se `other` for compatível com este revision:
    /// mesmo major e minor do peer ≤ minor local.
    pub fn compatible_with(&self, other: &Self) -> bool {
        self.major == other.major && other.minor <= self.minor
    }
}

// ---------------------------------------------------------------------------
// Identity types
// ---------------------------------------------------------------------------

/// Identidade textual de um peer remoto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerId(pub String);

impl PeerId {
    pub fn new(value: &str) -> Result<Self, ProtocolError> {
        if value.is_empty() || value.len() > MAX_ID_LEN || value.chars().any(char::is_control) {
            return Err(ProtocolError::InvalidIdentity);
        }
        Ok(Self(value.into()))
    }
}

/// Identidade textual de um node remoto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(value: &str) -> Result<Self, ProtocolError> {
        if value.is_empty() || value.len() > MAX_ID_LEN || value.chars().any(char::is_control) {
            return Err(ProtocolError::InvalidIdentity);
        }
        Ok(Self(value.into()))
    }
}

// ---------------------------------------------------------------------------
// Handshake
// ---------------------------------------------------------------------------

/// Handshake enviado pelo peer no início de uma conexão remota.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Handshake {
    pub protocol: ProtocolRevision,
    pub api: ProtocolRevision,
    pub peer: PeerId,
    pub node: NodeId,
    pub project: ProjectId,
    pub capabilities: BTreeSet<String>,
}

impl Handshake {
    /// Negocia este handshake contra as capacidades locais.
    ///
    /// Retorna [`NegotiatedProtocol`] quando:
    /// - Protocol major coincide e peer minor ≤ local minor.
    /// - API major coincide e peer minor ≤ local minor.
    /// - Todas as capabilities declaradas são conhecidas (intersection não
    ///   reduz o conjunto).
    ///
    /// Caso contrário retorna o erro correspondente: `UnsupportedProtocol`,
    /// `UnsupportedApi` ou `UnknownCapability`.
    pub fn negotiate(
        self,
        local_protocol: ProtocolRevision,
        local_api: ProtocolRevision,
        supported_capabilities: &BTreeSet<String>,
    ) -> Result<NegotiatedProtocol, ProtocolError> {
        if !local_protocol.compatible_with(&self.protocol) {
            return Err(ProtocolError::UnsupportedProtocol);
        }
        if !local_api.compatible_with(&self.api) {
            return Err(ProtocolError::UnsupportedApi);
        }
        let capabilities: BTreeSet<String> = self
            .capabilities
            .intersection(supported_capabilities)
            .cloned()
            .collect();
        if capabilities.len() != self.capabilities.len() {
            return Err(ProtocolError::UnknownCapability);
        }
        Ok(NegotiatedProtocol {
            protocol: self.protocol,
            api: self.api,
            peer: self.peer,
            node: self.node,
            project: self.project,
            capabilities,
        })
    }
}

/// Resultado de uma negociação de handshake bem-sucedida.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NegotiatedProtocol {
    pub protocol: ProtocolRevision,
    pub api: ProtocolRevision,
    pub peer: PeerId,
    pub node: NodeId,
    pub project: ProjectId,
    pub capabilities: BTreeSet<String>,
}

// ---------------------------------------------------------------------------
// Identity verification
// ---------------------------------------------------------------------------

/// Identidade esperada do peer remoto para verificação pré-handshake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedIdentity {
    peer: PeerId,
    node: NodeId,
}

impl ExpectedIdentity {
    pub fn new(peer: &str, node: &str) -> Result<Self, ProtocolError> {
        Ok(Self {
            peer: PeerId::new(peer)?,
            node: NodeId::new(node)?,
        })
    }

    /// Verifica se o handshake corresponde à identidade esperada.
    /// Peer e node devem coincidir exatamente.
    pub fn verify(&self, handshake: &Handshake) -> Result<(), ProtocolError> {
        if handshake.peer != self.peer || handshake.node != self.node {
            return Err(ProtocolError::IdentityMismatch);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Command catalog
// ---------------------------------------------------------------------------

/// Especificação de um comando do protocolo remoto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub name: String,
    /// `true` se o comando pode ser reenviado com o mesmo request_id
    /// sem efeito colateral adicional.
    pub idempotent: bool,
}

/// Catálogo tipado de comandos remotos conhecidos.
#[derive(Debug, Clone)]
pub struct CommandCatalog {
    entries: Vec<CommandSpec>,
}

impl CommandCatalog {
    /// Catálogo padrão da versão 1.0 do protocolo remoto.
    pub fn default_v1() -> Self {
        Self {
            entries: vec![
                CommandSpec {
                    name: "ping".into(),
                    idempotent: true,
                },
                CommandSpec {
                    name: "get_state".into(),
                    idempotent: true,
                },
                CommandSpec {
                    name: "subscribe".into(),
                    idempotent: false,
                },
                CommandSpec {
                    name: "cancel".into(),
                    idempotent: true,
                },
            ],
        }
    }

    /// Busca um comando pelo nome.
    ///
    /// Retorna [`ProtocolError::UnknownCommand`] se o comando não estiver
    /// registrado.
    pub fn lookup(&self, name: &str) -> Result<&CommandSpec, ProtocolError> {
        self.entries
            .iter()
            .find(|s| s.name == name)
            .ok_or(ProtocolError::UnknownCommand)
    }

    /// Lista os nomes de todos os comandos registrados.
    pub fn names(&self) -> Vec<&str> {
        self.entries.iter().map(|s| s.name.as_str()).collect()
    }
}

// ---------------------------------------------------------------------------
// Request tracker (correlation)
// ---------------------------------------------------------------------------

/// Estado de uma correlação de request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestState {
    Pending,
    Completed,
    Cancelled,
}

/// Rastreador bounded de correlações de request.
///
/// Rejeita:
/// - `begin` duplicado enquanto pending → `DuplicateCorrelation`
/// - `begin` em request_id terminal → `StaleCorrelation`
/// - `complete`/`cancel` em request_id desconhecido → `UnknownCorrelation`
/// - `complete` após cancel ou `cancel` após complete → `StaleCorrelation`
/// - Capacidade máxima excedida → `PayloadTooLarge`
#[derive(Debug, Clone)]
pub struct RequestTracker {
    states: BTreeMap<u64, RequestState>,
    max_correlations: usize,
}

impl RequestTracker {
    pub fn new(max_correlations: usize) -> Self {
        Self {
            states: BTreeMap::new(),
            max_correlations,
        }
    }

    /// Registra o início de um request.
    pub fn begin(&mut self, request_id: u64) -> Result<(), ProtocolError> {
        if request_id == 0 {
            return Err(ProtocolError::InvalidIdentity);
        }
        use std::collections::btree_map::Entry;
        if self.states.len() >= self.max_correlations {
            return Err(ProtocolError::PayloadTooLarge);
        }
        match self.states.entry(request_id) {
            Entry::Occupied(entry) => match entry.get() {
                RequestState::Pending => Err(ProtocolError::DuplicateCorrelation),
                RequestState::Completed | RequestState::Cancelled => {
                    Err(ProtocolError::StaleCorrelation)
                }
            },
            Entry::Vacant(entry) => {
                entry.insert(RequestState::Pending);
                Ok(())
            }
        }
    }

    /// Marca um request como completo.
    ///
    /// Idempotente se já completed: retorna `Ok`.
    /// Se pending, transiciona para completed.
    /// Se cancelled, retorna `StaleCorrelation`.
    /// Se desconhecido, retorna `UnknownCorrelation`.
    pub fn complete(&mut self, request_id: u64) -> Result<(), ProtocolError> {
        match self.states.get_mut(&request_id) {
            Some(state @ RequestState::Pending) => {
                *state = RequestState::Completed;
                Ok(())
            }
            Some(RequestState::Completed) => Ok(()), // idempotente
            Some(RequestState::Cancelled) => Err(ProtocolError::StaleCorrelation),
            None => Err(ProtocolError::UnknownCorrelation),
        }
    }

    /// Cancela um request pending.
    ///
    /// Se pending, transiciona para cancelled.
    /// Se já completed, retorna `StaleCorrelation`.
    /// Se desconhecido, retorna `UnknownCorrelation`.
    pub fn cancel(&mut self, request_id: u64) -> Result<(), ProtocolError> {
        match self.states.get_mut(&request_id) {
            Some(state @ RequestState::Pending) => {
                *state = RequestState::Cancelled;
                Ok(())
            }
            Some(RequestState::Cancelled) => Ok(()), // idempotente
            Some(RequestState::Completed) => Err(ProtocolError::StaleCorrelation),
            None => Err(ProtocolError::UnknownCorrelation),
        }
    }
}

// ---------------------------------------------------------------------------
// Event sequence
// ---------------------------------------------------------------------------

/// Rastreador de sequência de eventos para rejeitar mensagens
/// fora de ordem ou duplicadas.
#[derive(Debug, Clone, Copy)]
pub struct EventSequence {
    last: u64,
}

impl EventSequence {
    pub fn new() -> Self {
        Self { last: 0 }
    }

    /// Aceita um número de sequência se for estritamente maior que o último.
    ///
    /// Retorna `OutOfOrder` se `sequence ≤ last`.
    pub fn accept(&mut self, sequence: u64) -> Result<(), ProtocolError> {
        if sequence <= self.last {
            return Err(ProtocolError::OutOfOrder);
        }
        self.last = sequence;
        Ok(())
    }
}

impl Default for EventSequence {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Payload bound
// ---------------------------------------------------------------------------

/// Verificador de limite de payload.
pub struct PayloadBound;

impl PayloadBound {
    /// Retorna `Ok` se o payload estiver dentro do limite [`MAX_PAYLOAD`].
    pub fn check(payload: &[u8]) -> Result<(), ProtocolError> {
        if payload.len() > MAX_PAYLOAD {
            return Err(ProtocolError::PayloadTooLarge);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Error model
// ---------------------------------------------------------------------------

/// Erros do protocolo remoto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ProtocolError {
    #[error("unsupported protocol revision")]
    UnsupportedProtocol,
    #[error("unsupported API revision")]
    UnsupportedApi,
    #[error("unknown capability — capability is negotiated, not granted")]
    UnknownCapability,
    #[error("unknown command")]
    UnknownCommand,
    #[error("duplicate correlation — request already pending")]
    DuplicateCorrelation,
    #[error("stale correlation — request already completed or cancelled")]
    StaleCorrelation,
    #[error("unknown correlation")]
    UnknownCorrelation,
    #[error("out-of-order event sequence")]
    OutOfOrder,
    #[error("identity mismatch")]
    IdentityMismatch,
    #[error("payload exceeds maximum size")]
    PayloadTooLarge,
    #[error("invalid identity")]
    InvalidIdentity,
}
