use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Pending,
    Ready,
    Stopped,
    Quarantined,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleRequest {
    plugin_id: String,
    digest: String,
    api_revision: String,
    approved: bool,
    permission_granted: bool,
    restart_budget: u8,
}

impl LifecycleRequest {
    pub fn new(
        plugin_id: &str,
        digest: &str,
        api_revision: &str,
        approved: bool,
        permission_granted: bool,
        restart_budget: u8,
    ) -> Result<Self, LifecycleError> {
        if plugin_id.is_empty() || digest.is_empty() || api_revision.is_empty() {
            return Err(LifecycleError::InvalidIdentity);
        }
        Ok(Self {
            plugin_id: plugin_id.into(),
            digest: digest.into(),
            api_revision: api_revision.into(),
            approved,
            permission_granted,
            restart_budget,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleEvent {
    Start,
    Stop,
    Crash,
    Hang,
    Revoke,
    VersionMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum LifecycleError {
    #[error("invalid lifecycle identity")]
    InvalidIdentity,
    #[error("plugin is not approved")]
    NotApproved,
    #[error("plugin permission is not granted")]
    PermissionDenied,
    #[error("plugin API is unsupported")]
    ApiUnsupported,
    #[error("plugin is quarantined")]
    Quarantined,
    #[error("restart budget exhausted")]
    RestartLimit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginLifecycle {
    request: LifecycleRequest,
    state: LifecycleState,
    restart_count: u8,
}

impl PluginLifecycle {
    pub fn new(request: LifecycleRequest) -> Self {
        Self {
            request,
            state: LifecycleState::Pending,
            restart_count: 0,
        }
    }

    pub fn state(&self) -> LifecycleState {
        self.state
    }

    pub fn apply(&mut self, event: LifecycleEvent) -> Result<LifecycleState, LifecycleError> {
        match (self.state, event) {
            (LifecycleState::Quarantined, LifecycleEvent::Stop) => {
                self.state = LifecycleState::Stopped;
                Ok(self.state)
            }
            (LifecycleState::Quarantined, _) => Err(LifecycleError::Quarantined),
            (LifecycleState::Stopped, LifecycleEvent::Stop) => Ok(self.state),
            (LifecycleState::Stopped, _) => Err(LifecycleError::Quarantined),
            (_, LifecycleEvent::Stop) => {
                self.state = LifecycleState::Stopped;
                Ok(self.state)
            }
            (LifecycleState::Pending, LifecycleEvent::Start)
            | (LifecycleState::Ready, LifecycleEvent::Start) => self.start(),
            (LifecycleState::Ready, LifecycleEvent::Crash | LifecycleEvent::Hang) => {
                self.state = LifecycleState::Quarantined;
                Ok(self.state)
            }
            (LifecycleState::Ready, LifecycleEvent::Revoke) => {
                self.state = LifecycleState::Stopped;
                Ok(self.state)
            }
            (LifecycleState::Ready, LifecycleEvent::VersionMismatch) => {
                self.state = LifecycleState::Quarantined;
                Ok(self.state)
            }
            (LifecycleState::Pending, LifecycleEvent::Crash | LifecycleEvent::Hang) => {
                self.state = LifecycleState::Quarantined;
                Ok(self.state)
            }
            (LifecycleState::Pending, _) => Err(LifecycleError::NotApproved),
        }
    }

    fn start(&mut self) -> Result<LifecycleState, LifecycleError> {
        if self.request.api_revision != "api-1" {
            return Err(LifecycleError::ApiUnsupported);
        }
        if !self.request.approved {
            return Err(LifecycleError::NotApproved);
        }
        if !self.request.permission_granted {
            return Err(LifecycleError::PermissionDenied);
        }
        if self.restart_count >= self.request.restart_budget {
            return Err(LifecycleError::RestartLimit);
        }
        self.restart_count = self.restart_count.saturating_add(1);
        self.state = LifecycleState::Ready;
        Ok(self.state)
    }
}
