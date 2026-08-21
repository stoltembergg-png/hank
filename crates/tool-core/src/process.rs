//! Structured, non-shell process execution primitive.

use crate::{
    PermissionDecision, ToolCancellation, ToolExecutionStatus, ToolExecutionWindow,
    ToolTerminalState,
};
use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

pub const DEFAULT_MAX_OUTPUT_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct ProcessSpec {
    pub project_id: ProjectId,
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: BTreeMap<String, String>,
    pub allowed_programs: BTreeSet<PathBuf>,
    pub allowed_roots: Vec<PathBuf>,
    pub permission: PermissionDecision,
    pub timeout: Duration,
    pub max_output_bytes: usize,
    pub trace_id: TraceId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessResult {
    pub trace_id: TraceId,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub cancelled: bool,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProcessError {
    #[error("permission decision does not allow process execution")]
    PermissionDenied,
    #[error("project identity is required")]
    MissingProject,
    #[error("program is not allowlisted")]
    ProgramNotAllowed,
    #[error("shell execution is not allowed")]
    ShellNotAllowed,
    #[error("working directory is outside authorized roots")]
    CwdOutsideRoot,
    #[error("environment key is not allowed")]
    EnvironmentNotAllowed,
    #[error("process limits are invalid")]
    InvalidLimits,
    #[error("process failed to spawn")]
    SpawnFailed,
    #[error("process output is not valid UTF-8")]
    InvalidOutput,
}

impl ProcessSpec {
    pub fn validate(&self) -> Result<(), ProcessError> {
        if !self.permission.is_allowed() {
            return Err(ProcessError::PermissionDenied);
        }
        if self.allowed_programs.is_empty() || !self.allowed_programs.contains(&self.program) {
            return Err(ProcessError::ProgramNotAllowed);
        }
        if is_shell(&self.program) {
            return Err(ProcessError::ShellNotAllowed);
        }
        if self.allowed_roots.is_empty()
            || !self
                .allowed_roots
                .iter()
                .any(|root| self.cwd.starts_with(root))
        {
            return Err(ProcessError::CwdOutsideRoot);
        }
        if self.timeout.is_zero() || self.max_output_bytes == 0 {
            return Err(ProcessError::InvalidLimits);
        }
        for key in self.env.keys() {
            let lower = key.to_ascii_lowercase();
            if lower.contains("secret")
                || lower.contains("token")
                || lower.contains("password")
                || lower.contains("api_key")
            {
                return Err(ProcessError::EnvironmentNotAllowed);
            }
        }
        Ok(())
    }
}

pub fn run_process(
    spec: &ProcessSpec,
    cancel: Arc<AtomicBool>,
) -> Result<ProcessResult, ProcessError> {
    spec.validate()?;
    let window =
        ToolExecutionWindow::with_cancellation(spec.timeout, ToolCancellation::from_flag(cancel))
            .map_err(|_| ProcessError::InvalidLimits)?;
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .current_dir(&spec.cwd)
        .env_clear()
        .envs(&spec.env)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|_| ProcessError::SpawnFailed)?;
    loop {
        match window.poll() {
            ToolExecutionStatus::Terminal(ToolTerminalState::Cancelled) => {
                let output = terminate_child(child)?;
                return Ok(result_from_output(
                    spec.trace_id,
                    output,
                    false,
                    true,
                    spec.max_output_bytes,
                ));
            }
            ToolExecutionStatus::Terminal(ToolTerminalState::TimedOut) => {
                let output = terminate_child(child)?;
                return Ok(result_from_output(
                    spec.trace_id,
                    output,
                    true,
                    false,
                    spec.max_output_bytes,
                ));
            }
            ToolExecutionStatus::Terminal(ToolTerminalState::Completed) => {
                let output = terminate_child(child)?;
                return Ok(result_from_output(
                    spec.trace_id,
                    output,
                    false,
                    false,
                    spec.max_output_bytes,
                ));
            }
            ToolExecutionStatus::Active => {}
        }
        if child
            .try_wait()
            .map_err(|_| ProcessError::SpawnFailed)?
            .is_some()
        {
            let output = child
                .wait_with_output()
                .map_err(|_| ProcessError::SpawnFailed)?;
            let state = window.finish();
            return Ok(result_from_output(
                spec.trace_id,
                output,
                state == ToolTerminalState::TimedOut,
                state == ToolTerminalState::Cancelled,
                spec.max_output_bytes,
            ));
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn terminate_child(mut child: std::process::Child) -> Result<std::process::Output, ProcessError> {
    let _ = child.kill();
    child
        .wait_with_output()
        .map_err(|_| ProcessError::SpawnFailed)
}

fn result_from_output(
    trace_id: TraceId,
    output: std::process::Output,
    timed_out: bool,
    cancelled: bool,
    limit: usize,
) -> ProcessResult {
    let (stdout, stdout_truncated) = bounded_redacted(&output.stdout, limit);
    let (stderr, stderr_truncated) = bounded_redacted(&output.stderr, limit);
    ProcessResult {
        trace_id,
        exit_code: output.status.code(),
        stdout,
        stderr,
        timed_out,
        cancelled,
        stdout_truncated,
        stderr_truncated,
    }
}

fn bounded_redacted(bytes: &[u8], limit: usize) -> (String, bool) {
    let truncated = bytes.len() > limit;
    let text = String::from_utf8_lossy(&bytes[..bytes.len().min(limit)]);
    let redacted = text
        .lines()
        .map(|line| {
            let lower = line.to_ascii_lowercase();
            if ["secret", "token", "password", "api_key"]
                .iter()
                .any(|needle| lower.contains(needle))
            {
                "[redacted]"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    (redacted, truncated)
}

fn is_shell(program: &Path) -> bool {
    matches!(program.file_name().and_then(|name| name.to_str()).map(|name| name.to_ascii_lowercase()), Some(name) if ["sh", "bash", "zsh", "fish", "cmd", "powershell", "pwsh"].contains(&name.as_str()))
}
