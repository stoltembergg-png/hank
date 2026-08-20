//! Runtime de execução da plataforma multiagente.
//!
//! Esta crate contém a implementação do runtime: Agent Runtime,
//! Provider Port/Adapters, Tool Runtime, Sandbox Broker, Python Worker,
//! Memory, Skills, Workflow, Scheduler.
//!
//! Pode depender de: agent-core, agent-protocol, tokio, sqlx, tracing
//! NÃO deve vazar providers concretos para o core.

pub mod agent_repo;
pub mod agent_service;
pub mod event_bus;
pub mod memory;
pub mod migrations;
pub mod project_archive_service;
pub mod project_query_service;
pub mod project_repo;
pub mod project_service;
pub mod project_update_service;
pub mod provider;
pub mod python;
pub mod sandbox;
pub mod scheduler;
pub mod skill_runtime;
pub mod sqlite;
pub mod tool;
pub mod workflow_runtime;

pub use agent_core::*;
pub use agent_service::*;
pub use migrations::*;
pub use project_archive_service::*;
pub use project_query_service::*;
pub use project_repo::*;
pub use project_service::*;
pub use project_update_service::*;
pub use sqlite::*;
