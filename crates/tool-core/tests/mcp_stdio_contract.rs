use tool_core::mcp_stdio::*;

fn valid() -> StdioConfig {
    StdioConfig::new(
        "/usr/bin/mcp-server",
        vec!["--stdio".into()],
        vec!["/usr/bin".into()],
        256,
        1,
    )
    .unwrap()
}

// @spec:AC-1379
#[test]
fn declared_command_is_bounded_and_rejects_unsafe_values() {
    let config = valid();
    assert_eq!(config.executable(), "/usr/bin/mcp-server");
    assert_eq!(config.max_output(), 256);
    assert!(matches!(
        StdioConfig::new("mcp-server", vec![], vec!["/usr/bin".into()], 256, 1),
        Err(StdioError::ExecutableNotAllowed)
    ));
    assert!(matches!(
        StdioConfig::new(
            "/usr/bin/mcp-server",
            vec!["$(id)".into()],
            vec!["/usr/bin".into()],
            256,
            1
        ),
        Err(StdioError::ShellSyntax)
    ));
    assert!(matches!(
        config.accept_output(257),
        Err(StdioError::OutputTooLarge)
    ));
}

// @spec:AC-1380
#[test]
fn lifecycle_reasons_and_restart_limit_are_fail_closed() {
    let mut process = StdioProcess::new(valid());
    assert_eq!(process.cancel(), ProcessState::Cancelled);
    assert_eq!(process.cancel(), ProcessState::Cancelled);
    assert_eq!(process.record_crash(), ProcessState::Crashed);
    assert_eq!(process.restart(), Ok(ProcessState::Restarting));
    assert_eq!(process.restart(), Err(StdioError::RestartLimit));
    assert_eq!(process.timeout(), ProcessState::TimedOut);
}
