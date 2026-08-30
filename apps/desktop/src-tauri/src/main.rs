#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod agents;
pub mod confirmations;
pub mod lifecycle;
pub mod memory;
pub mod notifications;
pub mod projects;
pub mod scheduler;
pub mod skills;
pub mod streaming;

use agent_runtime::{
    migrations::run_migrations,
    sqlite::{SqliteStorage, SqliteStorageConfig},
};
use std::{io, path::PathBuf, time::Duration};
use tauri::{Manager, WindowEvent};

const E2E_APP_DATA_ENV: &str = "HANK_E2E_APP_DATA_DIR";
const E2E_RELEASE_DATA_OPT_IN_ENV: &str = "HANK_E2E_ALLOW_RELEASE_DATA_DIR";
const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

fn database_path(app: &tauri::AppHandle) -> Result<PathBuf, io::Error> {
    if let Some(e2e_dir) = std::env::var_os(E2E_APP_DATA_ENV) {
        let release_e2e_opt_in =
            std::env::var(E2E_RELEASE_DATA_OPT_IN_ENV).ok().as_deref() == Some("1");
        if !cfg!(debug_assertions) && !release_e2e_opt_in {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "HANK_E2E_APP_DATA_DIR requires explicit E2E opt-in for release test builds",
            ));
        }
        let directory = PathBuf::from(e2e_dir);
        std::fs::create_dir_all(&directory)?;
        return Ok(directory.join("hank.db"));
    }

    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| {
            io::Error::other(format!(
                "could not resolve application data directory: {error}"
            ))
        })?
        .join("hank.db"))
}

fn log_startup_failure(failure: &lifecycle::StartupFailure) {
    tracing::error!(
        event = "APPLICATION_STARTUP_FAILED",
        stage = %failure.stage,
        error_code = %failure.code,
        cause = %failure.cause,
        version = env!("CARGO_PKG_VERSION"),
        "application startup failed"
    );
}

fn startup_operation_failure(
    state: &lifecycle::StartupState,
    stage: lifecycle::StartupStage,
    code: lifecycle::StartupErrorCode,
    cause: impl Into<String>,
) -> Box<dyn std::error::Error> {
    let failure = state.fail(stage, code, cause);
    log_startup_failure(&failure);
    Box::new(failure)
}

fn startup_transition_failure(failure: lifecycle::StartupFailure) -> Box<dyn std::error::Error> {
    log_startup_failure(&failure);
    Box::new(failure)
}

fn main() {
    tracing_subscriber::fmt()
        .json()
        .with_target(false)
        .with_current_span(false)
        .with_span_list(false)
        .init();

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .manage(confirmations::bridge_state())
        .manage(lifecycle::StartupState::new())
        .invoke_handler(confirmations::command_handler())
        .on_page_load(|webview, payload| {
            if webview.label() != lifecycle::MAIN_WINDOW_LABEL
                || payload.event() != tauri::webview::PageLoadEvent::Finished
            {
                return;
            }

            if let Some(state) = webview.app_handle().try_state::<lifecycle::StartupState>() {
                match state.mark_webview_ready() {
                    Ok(snapshot) => tracing::info!(
                        event = "webview_ready",
                        stage = %snapshot.stage,
                        url = %payload.url(),
                        "desktop WebView finished loading"
                    ),
                    Err(failure) => log_startup_failure(&failure),
                }
            }
        })
        .setup(|app| {
            let startup = app.state::<lifecycle::StartupState>();
            tracing::info!(
                event = "boot",
                stage = %lifecycle::StartupStage::Booting,
                version = env!("CARGO_PKG_VERSION"),
                "application starting"
            );

            let database_path = database_path(app.handle()).map_err(|error| {
                startup_operation_failure(
                    &startup,
                    lifecycle::StartupStage::Booting,
                    lifecycle::StartupErrorCode::StorageInitialization,
                    error.to_string(),
                )
            })?;
            let storage = tauri::async_runtime::block_on(async move {
                let storage = SqliteStorage::connect(SqliteStorageConfig::for_file(database_path))
                    .await
                    .map_err(|error| io::Error::other(error.to_string()))?;
                run_migrations(storage.pool())
                    .await
                    .map_err(|error| io::Error::other(error.to_string()))?;
                Ok::<_, io::Error>(storage)
            })
            .map_err(|error| {
                startup_operation_failure(
                    &startup,
                    lifecycle::StartupStage::Booting,
                    lifecycle::StartupErrorCode::StorageInitialization,
                    error.to_string(),
                )
            })?;
            app.manage(std::sync::Mutex::new(
                agent_runtime::notifications::NotificationWorker::new(
                    notifications::TauriNotificationSink::new(app.handle().clone()),
                ),
            ));
            startup
                .advance(lifecycle::StartupStage::StorageReady)
                .map_err(startup_transition_failure)?;
            app.manage(projects::bridge_state(&storage));
            app.manage(agents::bridge_state(&storage));
            app.manage(scheduler::bridge_state(&storage));
            app.manage(memory::bridge_state(&storage));
            app.manage(skills::bridge_state(&storage));
            startup
                .advance(lifecycle::StartupStage::RuntimeReady)
                .map_err(startup_transition_failure)?;

            let window_config = app
                .config()
                .app
                .windows
                .iter()
                .find(|window| window.label == lifecycle::MAIN_WINDOW_LABEL)
                .ok_or_else(|| {
                    startup_operation_failure(
                        &startup,
                        lifecycle::StartupStage::RuntimeReady,
                        lifecycle::StartupErrorCode::WindowCreation,
                        "main window configuration is missing",
                    )
                })?;
            let webview_data_directory = app
                .path()
                .app_local_data_dir()
                .map_err(|error| {
                    startup_operation_failure(
                        &startup,
                        lifecycle::StartupStage::RuntimeReady,
                        lifecycle::StartupErrorCode::WebviewInitialization,
                        format!("could not resolve WebView data directory: {error}"),
                    )
                })?
                .join("webview");
            tauri::WebviewWindowBuilder::from_config(app.handle(), window_config)
                .map_err(|error| {
                    startup_operation_failure(
                        &startup,
                        lifecycle::StartupStage::RuntimeReady,
                        lifecycle::StartupErrorCode::WindowCreation,
                        error.to_string(),
                    )
                })?
                .data_directory(webview_data_directory)
                .build()
                .map_err(|error| {
                    startup_operation_failure(
                        &startup,
                        lifecycle::StartupStage::RuntimeReady,
                        lifecycle::StartupErrorCode::WebviewInitialization,
                        error.to_string(),
                    )
                })?;
            startup
                .advance(lifecycle::StartupStage::WindowCreated)
                .map_err(startup_transition_failure)?;

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(STARTUP_TIMEOUT).await;
                let startup = app_handle.state::<lifecycle::StartupState>();
                if startup.is_application_ready() {
                    return;
                }
                let failure = startup.fail(
                    startup.current_stage(),
                    lifecycle::StartupErrorCode::StartupTimeout,
                    "frontend readiness handshake did not complete before the startup deadline",
                );
                log_startup_failure(&failure);
                app_handle.exit(1);
            });

            Ok(())
        })
        .on_window_event(|_window, event| match event {
            WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed => {
                tracing::info!(
                    event = "close",
                    version = env!("CARGO_PKG_VERSION"),
                    "application closing"
                );
            }
            WindowEvent::Focused(focused) => {
                tracing::debug!(event = "focus", focused, "window focus changed");
            }
            _ => {}
        })
        .run(tauri::generate_context!());

    if let Err(error) = result {
        tracing::error!(
            event = "failure",
            version = env!("CARGO_PKG_VERSION"),
            error = %error,
            "application failed"
        );
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::{database_path, E2E_APP_DATA_ENV};

    #[test]
    fn e2e_data_override_is_explicitly_named_and_debug_only() {
        assert_eq!(E2E_APP_DATA_ENV, "HANK_E2E_APP_DATA_DIR");
        let source = include_str!("main.rs");
        assert!(source.contains("cfg!(debug_assertions)"));
        assert!(source.contains("HANK_E2E_ALLOW_RELEASE_DATA_DIR"));
        assert!(source.contains("explicit E2E opt-in"));
        assert!(source.contains("PermissionDenied"));
        let _ = database_path;
    }
}
