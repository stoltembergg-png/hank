pub mod confirmations;
pub mod streaming;

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
