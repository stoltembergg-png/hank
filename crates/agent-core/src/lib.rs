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
pub mod autonomy;
pub mod budget;
pub mod config;
pub mod dedupe;
pub mod embedding;
pub mod error;
pub mod importance;
pub mod keyword;
pub mod memory;
pub mod memory_candidate;
pub mod project;
pub mod session;
pub mod skill;
pub mod taxonomy;
pub mod tool_permissions;
pub mod workflow;

pub use agent::*;
pub use agent_protocol::{capability, events, ids, policy, version};
pub use agent_protocol::{
    Action, AgentId, AgentPolicyConfig, BudgetPolicy, Capability, CapabilitySet, EventId,
    EventKind, InstructionHierarchy, InstructionLayer, InstructionSource, MemoryId, MessageId,
    ModelPolicy, NodeId, PolicyDecision, ProjectId, ProtocolVersion, Resource, RunId, SessionId,
    SkillId, TaskId, ToolDefaultAction, ToolPolicy, TraceId, WorkflowId,
};
pub use autonomy::*;
pub use budget::*;
pub use dedupe::*;
pub use embedding::*;
pub use error::DomainError;
pub use error::DomainResult;
pub use importance::*;
pub use keyword::*;
pub use memory::*;
pub use memory_candidate::*;
pub use project::*;
pub use session::*;
pub use skill::*;
pub use taxonomy::*;
pub use workflow::*;
