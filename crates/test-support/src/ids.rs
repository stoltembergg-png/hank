//! Deterministic, offline ID fixtures (dev-only).
//!
//! Gera IDs tipados a partir de um seed, permitindo testes reproduzíveis
//! sem depender de UUIDs aleatórios. Nunca toca estado de produção.

use agent_protocol::ids::{
    AgentId, ArtifactId, CredentialId, GroupId, MemoryId, MessageId, NodeId,
    ProjectId, ProviderId, RequestId, RunId, SessionId, SkillId, TaskId,
    ToolId, TraceId, WorkflowId,
};
use agent_protocol::Uuid;

/// UUID determinístico a partir de um seed (dev-only).
pub fn seeded_uuid(seed: u64) -> Uuid {
    let mut bytes = [0u8; 16];
    bytes[0..8].copy_from_slice(&seed.to_be_bytes());
    Uuid::from_bytes(bytes)
}

pub fn project_id(seed: u64) -> ProjectId {
    ProjectId::from_uuid(seeded_uuid(seed))
}

pub fn agent_id(seed: u64) -> AgentId {
    AgentId::from_uuid(seeded_uuid(seed))
}

pub fn session_id(seed: u64) -> SessionId {
    SessionId::from_uuid(seeded_uuid(seed))
}

pub fn message_id(seed: u64) -> MessageId {
    MessageId::from_uuid(seeded_uuid(seed))
}

pub fn workflow_id(seed: u64) -> WorkflowId {
    WorkflowId::from_uuid(seeded_uuid(seed))
}

pub fn node_id(seed: u64) -> NodeId {
    NodeId::from_uuid(seeded_uuid(seed))
}

pub fn run_id(seed: u64) -> RunId {
    RunId::from_uuid(seeded_uuid(seed))
}

pub fn trace_id(seed: u64) -> TraceId {
    TraceId::from_uuid(seeded_uuid(seed))
}

pub fn request_id(seed: u64) -> RequestId {
    RequestId::from_uuid(seeded_uuid(seed))
}

pub fn skill_id(seed: u64) -> SkillId {
    SkillId::from_uuid(seeded_uuid(seed))
}

pub fn memory_id(seed: u64) -> MemoryId {
    MemoryId::from_uuid(seeded_uuid(seed))
}

pub fn tool_id(seed: u64) -> ToolId {
    ToolId::from_uuid(seeded_uuid(seed))
}

pub fn provider_id(seed: u64) -> ProviderId {
    ProviderId::from_uuid(seeded_uuid(seed))
}

pub fn credential_id(seed: u64) -> CredentialId {
    CredentialId::from_uuid(seeded_uuid(seed))
}

pub fn group_id(seed: u64) -> GroupId {
    GroupId::from_uuid(seeded_uuid(seed))
}

pub fn task_id(seed: u64) -> TaskId {
    TaskId::from_uuid(seeded_uuid(seed))
}

pub fn artifact_id(seed: u64) -> ArtifactId {
    ArtifactId::from_uuid(seeded_uuid(seed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_ids_are_deterministic_and_roundtrip() {
        let first = project_id(42);
        let second = project_id(42);
        assert_eq!(first, second);
        assert_ne!(first, project_id(43));
        assert_eq!(first, first.to_string().parse().unwrap());
        assert_eq!(seeded_uuid(7), seeded_uuid(7));
    }

    #[test]
    fn all_id_fixtures_generate_canonical_prefixes() {
        assert!(project_id(1).to_string().starts_with("proj-"));
        assert!(agent_id(1).to_string().starts_with("agent-"));
        assert!(session_id(1).to_string().starts_with("sess-"));
        assert!(message_id(1).to_string().starts_with("msg-"));
        assert!(workflow_id(1).to_string().starts_with("wf-"));
        assert!(node_id(1).to_string().starts_with("node-"));
        assert!(run_id(1).to_string().starts_with("run-"));
        assert!(trace_id(1).to_string().starts_with("trace-"));
        assert!(request_id(1).to_string().starts_with("req-"));
        assert!(skill_id(1).to_string().starts_with("skill-"));
        assert!(memory_id(1).to_string().starts_with("mem-"));
        assert!(tool_id(1).to_string().starts_with("tool-"));
        assert!(provider_id(1).to_string().starts_with("prov-"));
        assert!(credential_id(1).to_string().starts_with("cred-"));
        assert!(group_id(1).to_string().starts_with("grp-"));
        assert!(task_id(1).to_string().starts_with("task-"));
        assert!(artifact_id(1).to_string().starts_with("art-"));
    }
}
