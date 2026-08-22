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
pub mod cancellation;
pub mod chat_command;
pub mod confirmation_application;
pub mod context;
pub mod event_bus;
pub mod execution;
pub mod memory;
pub mod message_repo;
pub mod migrations;
pub mod project_archive_service;
pub mod project_query_service;
pub mod project_repo;
pub mod project_service;
pub mod project_update_service;
pub mod provider;
pub mod provider_service;
pub mod python;
pub mod python_executor;
pub mod python_lifecycle;
pub mod retry;
pub mod sandbox;
pub mod scheduler;
pub mod session_repo;
pub mod session_service;
pub mod skill_runtime;
pub mod sqlite;
pub mod streaming;
pub mod tool;
pub mod usage;
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
