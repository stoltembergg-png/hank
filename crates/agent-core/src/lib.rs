//! Domínio puro da plataforma multiagente.
//!
//! Esta crate contém apenas regras de domínio, entidades, invariantes,
//! policies puras, state machines, budgets e errors.
//!
//! REGRAS ARQUITETURAIS (AI-001, AI-003, D-001, D-002):
//! - NÃO importar: tauri, tao, wry, sqlx, tokio, providers concretos
//! - NÃO acessar: filesystem, network, secrets, env vars diretamente
//! - APENAS depender de: agent-protocol, std, serde, thiserror, uuid, chrono

pub mod agent;
pub mod budget;
pub mod config;
pub mod error;
pub mod memory;
pub mod project;
pub mod session;
pub mod skill;
pub mod workflow;

pub use agent::*;
pub use agent_protocol::*;
pub use budget::*;
pub use error::DomainError;
pub use error::DomainResult;
pub use memory::*;
pub use project::*;
pub use session::*;
pub use skill::*;
pub use workflow::*;
