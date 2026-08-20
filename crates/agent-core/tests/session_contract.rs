use agent_core::session::{Session, SessionError, SessionParticipant, SessionRole, SessionStatus};
use agent_protocol::ids::{AgentId, ProjectId, TraceId};
use serde_json::json;

fn project() -> ProjectId {
    ProjectId::new()
}

fn agent() -> AgentId {
    AgentId::new()
}

#[test]
fn session_requires_bounded_correlation_and_binds_project_agent() {
    let project_id = project();
    let agent_id = agent();
    let session = Session::new(project_id, agent_id, "corr_1").unwrap();
    assert_eq!(session.status, SessionStatus::Created);
    assert_eq!(session.project_id, project_id);
    assert_eq!(session.agent_id, agent_id);
    assert_eq!(session.correlation_id, "corr_1");
    assert!(Session::new(project_id, agent_id, "").is_err());
    assert!(Session::new(project_id, agent_id, "x".repeat(129)).is_err());
    assert!(Session::new(project_id, agent_id, "api_key=secret").is_err());
}

#[test]
fn lifecycle_is_deterministic_and_terminal_close_is_idempotent() {
    let mut session = Session::new(project(), agent(), "corr_1").unwrap();
    assert!(session.activate().is_ok());
    assert!(session.begin_close().is_ok());
    assert!(session.close().is_ok());
    assert_eq!(session.status, SessionStatus::Closed);
    assert!(session.close().is_ok());
    assert!(matches!(session.activate(), Err(SessionError::Terminal)));
    assert!(matches!(session.fail("late"), Err(SessionError::Terminal)));
}

#[test]
fn invalid_transitions_fail_without_mutating_state() {
    let mut session = Session::new(project(), agent(), "corr_1").unwrap();
    assert!(matches!(
        session.close(),
        Err(SessionError::InvalidTransition { .. })
    ));
    assert_eq!(session.status, SessionStatus::Created);
    assert!(session.begin_close().is_err());
    assert_eq!(session.status, SessionStatus::Created);
    session.activate().unwrap();
    session.fail("provider outage").unwrap();
    assert_eq!(session.status, SessionStatus::Failed);
    assert!(session.close().is_err());
}

#[test]
fn participants_are_project_scoped_bounded_and_deduplicated() {
    let project_id = project();
    let mut session = Session::new(project_id, agent(), "corr_1").unwrap();
    let participant =
        SessionParticipant::new(project_id, agent(), SessionRole::Observer, "ui").unwrap();
    session.add_participant(participant.clone()).unwrap();
    assert!(matches!(
        session.add_participant(participant),
        Err(SessionError::DuplicateParticipant)
    ));
    let foreign =
        SessionParticipant::new(project(), agent(), SessionRole::Observer, "foreign").unwrap();
    assert!(matches!(
        session.add_participant(foreign),
        Err(SessionError::ScopeMismatch)
    ));
    assert!(SessionParticipant::new(project_id, agent(), SessionRole::Observer, "").is_err());
    assert!(
        SessionParticipant::new(project_id, agent(), SessionRole::Observer, "x".repeat(129))
            .is_err()
    );
}

#[test]
fn references_are_bounded_redacted_and_not_prompt_storage() {
    let mut session = Session::new(project(), agent(), "corr_1").unwrap();
    session.set_budget_ref("budget_project_1").unwrap();
    session.set_trace_id(TraceId::new()).unwrap();
    assert_eq!(session.budget_ref.as_deref(), Some("budget_project_1"));
    assert!(session.set_budget_ref("token_secret").is_err());
    assert!(session.set_budget_ref("x".repeat(129)).is_err());
    let encoded = serde_json::to_string(&session).unwrap();
    assert!(!encoded.contains("prompt"));
    assert!(!format!("{session:?}").contains("token_secret"));
}

#[test]
fn serde_roundtrip_preserves_version_lifecycle_and_scope() {
    let mut session = Session::new(project(), agent(), "corr_1").unwrap();
    session.activate().unwrap();
    session.add_metadata("mode", json!("chat")).unwrap();
    let decoded: Session = serde_json::from_str(&serde_json::to_string(&session).unwrap()).unwrap();
    assert_eq!(decoded.schema_version, Session::SCHEMA_VERSION);
    assert_eq!(decoded.status, SessionStatus::Active);
    assert_eq!(decoded.project_id, session.project_id);
    assert_eq!(decoded.metadata.get("mode"), Some(&json!("chat")));
}

#[test]
fn metadata_and_budget_references_are_bounded() {
    let mut session = Session::new(project(), agent(), "corr_1").unwrap();
    assert!(session.add_metadata("key", json!("value")).is_ok());
    assert!(session.add_metadata("key", json!("new")).is_err());
    assert!(session.add_metadata("secret", json!("api_key=x")).is_err());
    assert!(session
        .add_metadata("x".repeat(129), json!("value"))
        .is_err());
    session.activate().unwrap();
    assert!(session.add_metadata("after", json!("active")).is_ok());
    session.begin_close().unwrap();
    assert!(session.add_metadata("late", json!("no")).is_err());
}
