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
pub mod agent_configuration_proposal;
pub mod architecture_profile;
pub mod automated_evaluation;
pub mod automatic_rollback;
pub mod automatic_skill_rollout;
pub mod autonomy;
pub mod budget;
pub mod ci_status_integration;
pub mod claim_evidence;
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
pub mod improvement_candidate;
pub mod improvement_observation;
pub mod improvement_scoring;
pub mod invocation_graph;
pub mod keyword;
pub mod mcp_transport;
pub mod memory;
pub mod memory_candidate;
pub mod memory_policy;
pub mod mention_parser;
pub mod moderator_policy;
pub mod parallel_batch;
pub mod parser;
pub mod planning_evidence_binding;
pub mod planning_reconciliation;
pub mod pr_generation_workflow;
pub mod project;
pub mod qa_profile;
pub mod regression_evaluation;
pub mod release_agent_workflow;
pub mod resource;
pub mod review_workflow;
pub mod reviewer_profile;
pub mod round_policy;
pub mod self_development_branch;
pub mod self_development_issue;
pub mod self_development_pr;
pub mod self_evaluation_workflow;
pub mod session;
pub mod skill;
pub mod skill_improvement_proposal;
pub mod synthesis;
pub mod task_mapping;
pub mod taxonomy;
pub mod tool_permissions;
pub mod vector;
pub mod versioning;
pub mod workflow;
pub mod workflow_improvement_proposal;
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
pub use claim_evidence::EvidenceStatus as ClaimEvidenceStatus;
pub use claim_evidence::{
    Claim, ClaimClass, ClaimError, ClaimEvidenceError, ClaimEvidenceKind, ClaimResolution,
    ClaimState, EvidenceRecord, EvidenceScope, FactState, ResolutionOutcome,
    CLAIM_EVIDENCE_SCHEMA_VERSION, MAX_CLAIM_EVIDENCE_REFERENCES, MAX_CLAIM_ID_LEN,
    MAX_EVIDENCE_RECORDS, MAX_REASON_LEN, MAX_REQUIRED_EVIDENCE, MAX_RESOLVER_ID_LEN,
    MAX_REVISION_LEN,
};
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
pub use planning_evidence_binding::*;
pub use planning_reconciliation::*;
pub use project::*;
pub use resource::*;
pub use round_policy::*;
pub use session::*;
pub use skill::*;
pub use synthesis::*;
pub use taxonomy::*;
pub use vector::*;
pub use versioning::*;
pub use workflow::*;
