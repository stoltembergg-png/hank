//! Runtime de execução da plataforma multiagente.
//!
//! Esta crate contém a implementação do runtime: Agent Runtime,
//! Provider Port/Adapters, Tool Runtime, Sandbox Broker, Python Worker,
//! Memory, Skills, Workflow, Scheduler.
//!
//! Pode depender de: agent-core, agent-protocol, tokio, sqlx, tracing
//! NÃO deve vazar providers concretos para o core.

pub mod agent_group_repo;
pub mod agent_node;
pub mod agent_repo;
pub mod agent_service;
pub mod agent_skills;
pub mod cancellation;
pub mod chat_command;
pub mod confirmation_application;
pub mod context;
pub mod event_bus;
pub mod execution;
pub mod memory;
pub mod memory_policy_repo;
pub mod memory_repo;
pub mod memory_service;
pub mod message_repo;
pub mod migrations;
pub mod project_archive_service;
pub mod project_query_service;
pub mod project_repo;
pub mod project_service;
pub mod project_skills;
pub mod project_update_service;
pub mod provider;
pub mod provider_service;
pub mod python;
pub mod python_environment;
pub mod python_executor;
pub mod python_lifecycle;
pub mod python_logs;
pub mod retry;
pub mod sandbox;
pub mod scheduler;
pub mod session_repo;
pub mod session_service;
pub mod skill_activation_policy;
pub mod skill_autonomous_test;
pub mod skill_candidate;
pub mod skill_creation;
pub mod skill_editor;
pub mod skill_evaluation;
pub mod skill_lifecycle_curator;
pub mod skill_loader;
pub mod skill_repo;
pub mod skill_rollback;
pub mod skill_runtime;
pub mod skill_testing;
pub mod skill_validation;
pub mod sqlite;
pub mod streaming;
pub mod tool;
pub mod usage;
pub mod workflow_repo;
pub mod workflow_runtime;

pub use agent_core::*;
pub use agent_group_repo::*;
pub use agent_service::*;
pub use agent_skills::*;
pub use memory_repo::*;
pub use migrations::*;
pub use project_archive_service::*;
pub use project_query_service::*;
pub use project_repo::*;
pub use project_service::*;
pub use project_skills::*;
pub use project_update_service::*;
pub use python_environment::*;
pub use skill_candidate::{
    SkillCandidate, SkillCandidateGenerationService, SkillCandidatePolicy, SkillCandidateProposal,
    SkillCandidateReason, SkillCandidateRequest, SkillCandidateStatus, SkillEvaluationHandoff,
    SkillObservationRef, SKILL_CANDIDATE_GENERATE_CAPABILITY, SKILL_CANDIDATE_SCHEMA_VERSION,
};
pub use skill_creation::{
    SkillCreateTool, SkillCreationPolicy, SkillCreationRequest, SkillCreationResult,
    SkillCreationService, SKILL_CREATE_CAPABILITY, SKILL_CREATE_TOOL_NAME,
    SKILL_CREATE_TOOL_VERSION,
};
pub use skill_editor::*;
pub use skill_evaluation::{
    SkillEvaluationPolicy, SkillEvaluationReason, SkillEvaluationReport, SkillEvaluationRequest,
    SkillEvaluationService, SkillEvaluationStatus, SKILL_EVALUATE_CAPABILITY,
    SKILL_EVALUATION_SCHEMA_VERSION,
};
pub use skill_loader::*;
pub use skill_repo::*;
pub use sqlite::*;
