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
pub mod architecture_profile;
pub mod autonomy;
pub mod budget;
pub mod ci_status_integration;
pub mod coding_profile;
pub mod config;
pub mod cycle_detection;
pub mod dedupe;
pub mod depth_limit;
pub mod embedding;
pub mod error;
pub mod fix_review_workflow;
pub mod group_budget;
pub mod group_entity;
pub mod group_session;
pub mod importance;
pub mod invocation_graph;
pub mod keyword;
pub mod memory;
pub mod memory_candidate;
pub mod memory_policy;
pub mod mention_parser;
pub mod moderator_policy;
pub mod parallel_batch;
pub mod parser;
pub mod pr_generation_workflow;
pub mod project;
pub mod qa_profile;
pub mod review_workflow;
pub mod reviewer_profile;
pub mod round_policy;
pub mod session;
pub mod skill;
pub mod synthesis;
pub mod task_mapping;
pub mod taxonomy;
pub mod tool_permissions;
pub mod vector;
pub mod versioning;
pub mod workflow;
pub mod workspace;
pub mod worktree;

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
pub use cycle_detection::*;
pub use dedupe::*;
pub use depth_limit::*;
pub use embedding::*;
pub use error::DomainError;
pub use error::DomainResult;
pub use group_budget::*;
pub use group_entity::*;
pub use group_session::*;
pub use importance::*;
pub use invocation_graph::*;
pub use keyword::*;
pub use memory::*;
pub use memory_candidate::*;
pub use memory_policy::*;
pub use mention_parser::*;
pub use moderator_policy::*;
pub use parallel_batch::*;
pub use parser::*;
pub use project::*;
pub use round_policy::*;
pub use session::*;
pub use skill::*;
pub use synthesis::*;
pub use taxonomy::*;
pub use vector::*;
pub use versioning::*;
pub use workflow::*;
