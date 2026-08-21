use agent_protocol::chat_command::{
    CallerIdentity, ChatCommand, ChatCommandError, ChatCommandRegistry, ChatCommandStatus,
};
use agent_protocol::ids::{AgentId, ProjectId, SessionId};

fn command() -> ChatCommand {
    ChatCommand::new(
        "command-1",
        CallerIdentity::new("caller-1", "desktop").unwrap(),
        ProjectId::new(),
        AgentId::new(),
        SessionId::new(),
        "hello",
        1,
        "cancel-1",
    )
    .unwrap()
}

#[test]
fn valid_command_is_versioned_bounded_and_bound_to_typed_identity() {
    let command = command();
    assert_eq!(command.schema_version, 1);
    assert_eq!(command.status(), ChatCommandStatus::Accepted);
    assert_eq!(command.text, "hello");
    let debug = format!("{command:?}");
    assert!(!debug.contains("api_key"));
    assert!(!debug.contains("secret"));
}

#[test]
fn registry_rejects_duplicate_and_stale_generation_deterministically() {
    let registry = ChatCommandRegistry::new(4).unwrap();
    let first = command();
    assert_eq!(
        registry.accept(&first).unwrap(),
        ChatCommandStatus::Accepted
    );
    assert_eq!(
        registry.accept(&first).unwrap(),
        ChatCommandStatus::Duplicate
    );
    let high = ChatCommand::new(
        "command-high",
        first.caller.clone(),
        first.project_id,
        first.agent_id,
        first.session_id,
        "high",
        2,
        "cancel-high",
    )
    .unwrap();
    assert_eq!(registry.accept(&high).unwrap(), ChatCommandStatus::Accepted);
    let stale = ChatCommand::new(
        "command-2",
        first.caller.clone(),
        first.project_id,
        first.agent_id,
        first.session_id,
        "next",
        1,
        "cancel-2",
    )
    .unwrap();
    assert_eq!(registry.accept(&stale).unwrap(), ChatCommandStatus::Stale);
}

#[test]
fn malformed_unknown_oversized_and_secret_like_commands_fail_closed() {
    assert!(matches!(
        ChatCommand::new(
            "",
            CallerIdentity::new("caller", "desktop").unwrap(),
            ProjectId::new(),
            AgentId::new(),
            SessionId::new(),
            "x",
            1,
            "cancel"
        ),
        Err(ChatCommandError::Invalid)
    ));
    assert!(ChatCommand::new(
        "command-1",
        CallerIdentity::new("caller", "desktop").unwrap(),
        ProjectId::new(),
        AgentId::new(),
        SessionId::new(),
        "x".repeat(1_048_577),
        1,
        "cancel"
    )
    .is_err());
    assert!(ChatCommand::new(
        "command-1",
        CallerIdentity::new("caller", "desktop").unwrap(),
        ProjectId::new(),
        AgentId::new(),
        SessionId::new(),
        "api_key=secret",
        1,
        "cancel"
    )
    .is_err());
    assert!(CallerIdentity::new("", "desktop").is_err());
}

#[test]
fn capacity_and_cancellation_metadata_are_bounded() {
    let registry = ChatCommandRegistry::new(1).unwrap();
    let first = command();
    registry.accept(&first).unwrap();
    let second = ChatCommand::new(
        "command-2",
        first.caller.clone(),
        first.project_id,
        first.agent_id,
        first.session_id,
        "second",
        1,
        "cancel-2",
    )
    .unwrap();
    assert!(matches!(
        registry.accept(&second),
        Err(ChatCommandError::Capacity)
    ));
    assert_eq!(first.cancellation_id, "cancel-1");
}
