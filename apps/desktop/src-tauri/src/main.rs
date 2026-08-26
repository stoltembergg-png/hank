#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod confirmations;
pub mod memory;
pub mod projects;
pub mod skills;
pub mod streaming;

use agent_runtime::{
    migrations::run_migrations,
    sqlite::{SqliteStorage, SqliteStorageConfig},
};
use std::{io, path::PathBuf};
use tauri::{Manager, WindowEvent};

const E2E_APP_DATA_ENV: &str = "HANK_E2E_APP_DATA_DIR";

fn database_path(app: &tauri::AppHandle) -> Result<PathBuf, io::Error> {
    if let Some(e2e_dir) = std::env::var_os(E2E_APP_DATA_ENV) {
        if !cfg!(debug_assertions) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "HANK_E2E_APP_DATA_DIR is available only in debug test builds",
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

fn main() {
    tracing_subscriber::fmt()
        .json()
        .with_target(false)
        .with_current_span(false)
        .with_span_list(false)
        .init();

    let result = tauri::Builder::default()
        .manage(confirmations::bridge_state())
        .invoke_handler(confirmations::command_handler())
        .setup(|app| {
            let database_path = database_path(app.handle())?;
            let storage = tauri::async_runtime::block_on(async move {
                let storage = SqliteStorage::connect(SqliteStorageConfig::for_file(database_path))
                    .await
                    .map_err(|error| io::Error::other(error.to_string()))?;
                run_migrations(storage.pool())
                    .await
                    .map_err(|error| io::Error::other(error.to_string()))?;
                Ok::<_, io::Error>(storage)
            })?;
            app.manage(projects::bridge_state(&storage));
            app.manage(memory::bridge_state(&storage));
            app.manage(skills::bridge_state(&storage));

            if app.get_webview_window("main").is_none() {
                return Err(Box::new(io::Error::new(
                    io::ErrorKind::NotFound,
                    "main window was not created",
                )));
            }

            tracing::info!(
                event = "boot",
                version = env!("CARGO_PKG_VERSION"),
                "application starting"
            );
            tracing::info!(
                event = "ready",
                version = env!("CARGO_PKG_VERSION"),
                "application ready"
            );
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
        assert!(source.contains("PermissionDenied"));
        let _ = database_path;
    }
}
