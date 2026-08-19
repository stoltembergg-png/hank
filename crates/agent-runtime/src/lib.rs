//! Runtime de execução da plataforma multiagente.
//!
//! Esta crate contém a implementação do runtime: Agent Runtime,
//! Provider Port/Adapters, Tool Runtime, Sandbox Broker, Python Worker,
//! Memory, Skills, Workflow, Scheduler.
//!
//! Pode depender de: agent-core, agent-protocol, tokio, sqlx, tracing
//! NÃO deve vazar providers concretos para o core.

pub mod memory;
pub mod provider;
pub mod python;
pub mod sandbox;
pub mod scheduler;
pub mod skill_runtime;
pub mod tool;
pub mod workflow_runtime;

pub use agent_core::*;
