#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod confirmations;
pub mod memory;
pub mod skills;
pub mod streaming;

use agent_runtime::{
    migrations::run_migrations,
    sqlite::{SqliteStorage, SqliteStorageConfig},
};
use tauri::{Manager, WindowEvent};

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
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let database_path = data_dir.join("hank.db");
            let storage = tauri::async_runtime::block_on(async move {
                let storage = SqliteStorage::connect(SqliteStorageConfig::for_file(database_path))
                    .await
                    .map_err(|error| std::io::Error::other(error.to_string()))?;
                run_migrations(storage.pool())
                    .await
                    .map_err(|error| std::io::Error::other(error.to_string()))?;
                Ok::<_, std::io::Error>(storage)
            })?;
            app.manage(memory::bridge_state(&storage));
            app.manage(skills::bridge_state(&storage));

            if app.get_webview_window("main").is_none() {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
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
