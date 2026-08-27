use std::sync::Mutex;

use serde::Serialize;
use tauri::State;

pub const MAIN_WINDOW_LABEL: &str = "main";

/// Ordered desktop boot milestones. `ApplicationReady` is only reachable after
/// the frontend has successfully crossed the real Tauri IPC boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StartupStage {
    Booting,
    StorageReady,
    RuntimeReady,
    WindowCreated,
    WebviewReady,
    FrontendReady,
    ApplicationReady,
}

impl StartupStage {
    fn next(self) -> Option<Self> {
        match self {
            Self::Booting => Some(Self::StorageReady),
            Self::StorageReady => Some(Self::RuntimeReady),
            Self::RuntimeReady => Some(Self::WindowCreated),
            Self::WindowCreated => Some(Self::WebviewReady),
            Self::WebviewReady => Some(Self::FrontendReady),
            Self::FrontendReady => Some(Self::ApplicationReady),
            Self::ApplicationReady => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StartupErrorCode {
    InvalidTransition,
    StorageInitialization,
    RuntimeInitialization,
    WindowCreation,
    WebviewInitialization,
    FrontendHandshake,
    StartupTimeout,
}

impl std::fmt::Display for StartupStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(self)
                .unwrap_or_default()
                .trim_matches('"')
        )
    }
}

impl std::fmt::Display for StartupErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(self)
                .unwrap_or_default()
                .trim_matches('"')
        )
    }
}

/// Bounded, structured startup failure returned to the frontend and recorded
/// in logs. The caller is responsible for passing a safe cause, never raw
/// user payloads or secrets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StartupFailure {
    pub stage: StartupStage,
    pub code: StartupErrorCode,
    pub cause: String,
}

impl std::fmt::Display for StartupFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "startup failed at {} ({})", self.stage, self.code)
    }
}

impl std::error::Error for StartupFailure {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupSnapshot {
    pub stage: StartupStage,
    pub ready_emitted: bool,
    pub failure: Option<StartupFailure>,
}

#[derive(Debug)]
struct StartupProgress {
    stage: StartupStage,
    ready_emitted: bool,
    failure: Option<StartupFailure>,
}

/// Thread-safe lifecycle state shared by setup, WebView callbacks and the
/// typed frontend handshake command.
pub struct StartupState {
    progress: Mutex<StartupProgress>,
}

impl StartupState {
    pub fn new() -> Self {
        Self {
            progress: Mutex::new(StartupProgress {
                stage: StartupStage::Booting,
                ready_emitted: false,
                failure: None,
            }),
        }
    }

    pub fn snapshot(&self) -> StartupSnapshot {
        let progress = self.progress.lock().expect("startup state mutex poisoned");
        StartupSnapshot {
            stage: progress.stage,
            ready_emitted: progress.ready_emitted,
            failure: progress.failure.clone(),
        }
    }

    pub fn current_stage(&self) -> StartupStage {
        self.snapshot().stage
    }

    pub fn advance(&self, next: StartupStage) -> Result<StartupSnapshot, StartupFailure> {
        let mut progress = self.progress.lock().expect("startup state mutex poisoned");
        if let Some(failure) = progress.failure.clone() {
            return Err(failure);
        }

        if progress.stage.next() != Some(next) {
            let failure = StartupFailure {
                stage: progress.stage,
                code: StartupErrorCode::InvalidTransition,
                cause: format!("expected {:?}, attempted {:?}", progress.stage.next(), next),
            };
            progress.failure = Some(failure.clone());
            return Err(failure);
        }

        progress.stage = next;
        progress.ready_emitted = next == StartupStage::ApplicationReady;
        Ok(StartupSnapshot {
            stage: progress.stage,
            ready_emitted: progress.ready_emitted,
            failure: None,
        })
    }

    pub fn mark_webview_ready(&self) -> Result<StartupSnapshot, StartupFailure> {
        let snapshot = self.snapshot();
        if let Some(failure) = snapshot.failure.as_ref() {
            return Err(failure.clone());
        }
        if matches!(
            snapshot.stage,
            StartupStage::WebviewReady
                | StartupStage::FrontendReady
                | StartupStage::ApplicationReady
        ) {
            return Ok(snapshot);
        }
        self.advance(StartupStage::WebviewReady)
    }

    pub fn acknowledge_frontend(&self) -> Result<StartupSnapshot, StartupFailure> {
        let snapshot = self.snapshot();
        if let Some(failure) = snapshot.failure.as_ref() {
            return Err(failure.clone());
        }
        if snapshot.stage == StartupStage::ApplicationReady {
            return Ok(snapshot);
        }
        // In release builds the frontend IPC callback can arrive before the
        // runtime's asynchronous PageLoadEvent::Finished callback. A
        // successful typed command is itself proof that the WebView loaded
        // enough to execute frontend code and cross the IPC boundary.
        if snapshot.stage == StartupStage::WindowCreated {
            self.advance(StartupStage::WebviewReady)?;
        }
        self.advance(StartupStage::FrontendReady)?;
        self.advance(StartupStage::ApplicationReady)
    }

    pub fn fail(
        &self,
        stage: StartupStage,
        code: StartupErrorCode,
        cause: impl Into<String>,
    ) -> StartupFailure {
        let mut progress = self.progress.lock().expect("startup state mutex poisoned");
        if let Some(failure) = progress.failure.clone() {
            return failure;
        }
        let failure = StartupFailure {
            stage,
            code,
            cause: cause.into(),
        };
        progress.failure = Some(failure.clone());
        progress.ready_emitted = false;
        failure
    }

    pub fn is_application_ready(&self) -> bool {
        let progress = self.progress.lock().expect("startup state mutex poisoned");
        progress.stage == StartupStage::ApplicationReady
            && progress.failure.is_none()
            && progress.ready_emitted
    }
}

impl Default for StartupState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FrontendReadyResponse {
    pub stage: StartupStage,
}

/// Typed command called by the actual bundled frontend after React mounted.
/// Its successful response is the final IPC proof required for readiness.
#[tauri::command]
pub fn frontend_ready(
    window: tauri::WebviewWindow,
    state: State<'_, StartupState>,
) -> Result<FrontendReadyResponse, StartupFailure> {
    if window.label() != MAIN_WINDOW_LABEL {
        return Err(state.fail(
            StartupStage::WindowCreated,
            StartupErrorCode::FrontendHandshake,
            "frontend readiness was requested by an unexpected window",
        ));
    }

    let snapshot = state.acknowledge_frontend().map_err(|failure| {
        state.fail(
            failure.stage,
            StartupErrorCode::FrontendHandshake,
            failure.cause,
        )
    })?;

    tracing::info!(
        event = "ready",
        stage = %snapshot.stage,
        version = env!("CARGO_PKG_VERSION"),
        "application ready"
    );

    Ok(FrontendReadyResponse {
        stage: snapshot.stage,
    })
}

#[cfg(test)]
mod tests {
    use super::{StartupErrorCode, StartupStage, StartupState};

    #[test]
    fn application_ready_requires_every_stage_and_frontend_ipc() {
        let state = StartupState::new();

        for stage in [
            StartupStage::StorageReady,
            StartupStage::RuntimeReady,
            StartupStage::WindowCreated,
        ] {
            state.advance(stage).expect("mandatory stage must advance");
            assert!(!state.snapshot().ready_emitted);
        }

        state
            .mark_webview_ready()
            .expect("WebView page load must advance readiness");
        assert!(!state.snapshot().ready_emitted);

        state
            .acknowledge_frontend()
            .expect("frontend IPC handshake must advance readiness");
        let snapshot = state.snapshot();
        assert_eq!(snapshot.stage, StartupStage::ApplicationReady);
        assert!(snapshot.ready_emitted);
    }

    #[test]
    fn webview_failure_is_fail_closed_and_never_emits_ready() {
        let state = StartupState::new();
        for stage in [
            StartupStage::StorageReady,
            StartupStage::RuntimeReady,
            StartupStage::WindowCreated,
        ] {
            state.advance(stage).expect("mandatory stage must advance");
        }

        let failure = state.fail(
            StartupStage::WindowCreated,
            StartupErrorCode::WebviewInitialization,
            "WebView2 returned HRESULT 0x800700AA",
        );
        assert_eq!(failure.stage, StartupStage::WindowCreated);
        assert_eq!(failure.code, StartupErrorCode::WebviewInitialization);
        assert!(state.mark_webview_ready().is_err());
        assert!(state.acknowledge_frontend().is_err());
        assert!(!state.snapshot().ready_emitted);
        assert!(state.snapshot().failure.is_some());
    }

    #[test]
    fn frontend_ipc_handshake_can_complete_before_page_load_callback() {
        let state = StartupState::new();
        for stage in [
            StartupStage::StorageReady,
            StartupStage::RuntimeReady,
            StartupStage::WindowCreated,
        ] {
            state.advance(stage).expect("mandatory stage must advance");
        }

        let snapshot = state
            .acknowledge_frontend()
            .expect("typed IPC proves the loaded WebView even before callback ordering settles");
        assert_eq!(snapshot.stage, StartupStage::ApplicationReady);
        assert!(snapshot.ready_emitted);
        assert!(state.mark_webview_ready().is_ok());
    }
}
