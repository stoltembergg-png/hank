//! Bounded stdio adapter contract; concrete spawning is owned by the process boundary.
const MAX_TEXT: usize = 256;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdioConfig {
    executable: String,
    args: Vec<String>,
    roots: Vec<String>,
    output: usize,
    restart_limit: u8,
}
impl StdioConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        executable: &str,
        args: Vec<String>,
        roots: Vec<String>,
        output: usize,
        restart_limit: u8,
    ) -> Result<Self, StdioError> {
        if !executable.starts_with('/')
            || executable.len() > MAX_TEXT
            || !roots.iter().any(|r| executable.starts_with(r))
        {
            return Err(StdioError::ExecutableNotAllowed);
        }
        if args.iter().any(|a| {
            a.len() > MAX_TEXT || a.chars().any(|c| matches!(c, '$' | '`' | ';' | '|' | '&'))
        }) {
            return Err(StdioError::ShellSyntax);
        }
        if output == 0 || output > 1024 * 1024 {
            return Err(StdioError::OutputTooLarge);
        }
        Ok(Self {
            executable: executable.into(),
            args,
            roots,
            output,
            restart_limit,
        })
    }
    pub fn executable(&self) -> &str {
        &self.executable
    }
    pub fn max_output(&self) -> usize {
        self.output
    }
    pub fn accept_output(&self, size: usize) -> Result<(), StdioError> {
        if size > self.output {
            Err(StdioError::OutputTooLarge)
        } else {
            Ok(())
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdioError {
    ExecutableNotAllowed,
    ShellSyntax,
    OutputTooLarge,
    RestartLimit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Starting,
    Running,
    Cancelled,
    Crashed,
    Restarting,
    TimedOut,
}
pub struct StdioProcess {
    config: StdioConfig,
    state: ProcessState,
    restarts: u8,
}
impl StdioProcess {
    pub fn new(config: StdioConfig) -> Self {
        Self {
            config,
            state: ProcessState::Starting,
            restarts: 0,
        }
    }
    pub fn cancel(&mut self) -> ProcessState {
        self.state = ProcessState::Cancelled;
        self.state
    }
    pub fn record_crash(&mut self) -> ProcessState {
        self.state = ProcessState::Crashed;
        self.state
    }
    pub fn restart(&mut self) -> Result<ProcessState, StdioError> {
        if self.restarts >= self.config.restart_limit {
            return Err(StdioError::RestartLimit);
        }
        self.restarts += 1;
        self.state = ProcessState::Restarting;
        Ok(self.state)
    }
    pub fn timeout(&mut self) -> ProcessState {
        self.state = ProcessState::TimedOut;
        self.state
    }
}
