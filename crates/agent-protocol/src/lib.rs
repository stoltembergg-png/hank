//! Tipos estáveis, serialização e contratos de protocolo da plataforma multiagente.
//!
//! Esta crate define os tipos que cruzam fronteiras arquiteturais:
//! - IDs tipados (ProjectId, AgentId, SessionId, etc.)
//! - Envelopes de comando/resultado/evento
//! - Schemas de capability, permission, policy
//! - Versões de protocolo e compatibilidade

pub mod capability;
pub mod chat_command;
pub mod chat_stream;
pub mod envelope;
pub mod events;
pub mod ids;
pub mod invocation;
pub mod json_rpc;
pub mod policy;
pub mod runtime_transport;
pub mod version;
pub mod worker;

pub use capability::*;
pub use envelope::*;
pub use events::*;
pub use ids::*;
pub use invocation::*;
pub use policy::*;
pub use version::*;

/// Versão atual do protocolo para negociação de compatibilidade
pub const PROTOCOL_VERSION: &str = env!("CARGO_PKG_VERSION");

pub use chrono::{DateTime, Utc};
pub use serde::{Deserialize, Serialize};
pub use thiserror::Error;
/// Re-export tipos comuns para conveniência
pub use uuid::Uuid;
