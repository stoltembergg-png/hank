use agent_runtime::notifications::{
    DeliveryOutcome, Notification, NotificationDecision, NotificationSink, NotificationWorker,
    PermissionState as RuntimePermissionState,
};
use std::sync::Mutex;
use tauri::{AppHandle, Runtime};
use tauri_plugin_notification::{NotificationExt, PermissionState as TauriPermissionState};

/// Adapter-only implementation. Domain/runtime code depends only on NotificationSink.
pub struct TauriNotificationSink<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriNotificationSink<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }

    pub fn request_permission(&self) -> RuntimePermissionState {
        match self.app.notification().request_permission() {
            Ok(state) => map_permission(state),
            Err(error) => {
                tracing::warn!(event = "notification_permission_error", error = %error);
                RuntimePermissionState::Unavailable
            }
        }
    }
}

impl<R: Runtime> NotificationSink for TauriNotificationSink<R> {
    fn permission(&self) -> RuntimePermissionState {
        match self.app.notification().permission_state() {
            Ok(state) => map_permission(state),
            Err(error) => {
                tracing::warn!(event = "notification_permission_state_error", error = %error);
                RuntimePermissionState::Unavailable
            }
        }
    }

    fn deliver(&mut self, notification: &Notification) -> bool {
        let result = self
            .app
            .notification()
            .builder()
            .title(notification.title.clone())
            .body(notification.body.clone())
            .show();
        if let Err(error) = result {
            tracing::warn!(
                event = "notification_delivery_error",
                project_id = %notification.project_id,
                run_id = %notification.run_id,
                error = %error,
            );
            return false;
        }
        tracing::info!(
            event = "notification_delivered",
            project_id = %notification.project_id,
            run_id = %notification.run_id,
            severity = %notification.severity,
        );
        true
    }
}

pub type TauriNotificationWorker<R> = Mutex<NotificationWorker<TauriNotificationSink<R>>>;

pub fn process_decision<R: Runtime>(
    worker: &TauriNotificationWorker<R>,
    decision: NotificationDecision,
) -> DeliveryOutcome {
    match decision {
        NotificationDecision::Suppressed(reason) => DeliveryOutcome::Suppressed(reason),
        NotificationDecision::Deliver(notification) => match worker.lock() {
            Ok(mut worker) => worker.deliver(&notification),
            Err(error) => {
                tracing::warn!(event = "notification_worker_poisoned", error = %error);
                DeliveryOutcome::Failed
            }
        },
    }
}

fn map_permission(state: TauriPermissionState) -> RuntimePermissionState {
    match state {
        TauriPermissionState::Granted => RuntimePermissionState::Granted,
        TauriPermissionState::Denied
        | TauriPermissionState::Prompt
        | TauriPermissionState::PromptWithRationale => RuntimePermissionState::Denied,
    }
}

#[cfg(test)]
mod tests {
    use super::map_permission;
    use agent_runtime::notifications::PermissionState;
    use tauri_plugin_notification::PermissionState as TauriPermissionState;

    #[test]
    fn maps_plugin_permission_states_without_leaking_plugin_types() {
        assert_eq!(
            map_permission(TauriPermissionState::Granted),
            PermissionState::Granted
        );
        assert_eq!(
            map_permission(TauriPermissionState::Denied),
            PermissionState::Denied
        );
        assert_eq!(
            map_permission(TauriPermissionState::Prompt),
            PermissionState::Denied
        );
        assert_eq!(
            map_permission(TauriPermissionState::PromptWithRationale),
            PermissionState::Denied
        );
    }
}
