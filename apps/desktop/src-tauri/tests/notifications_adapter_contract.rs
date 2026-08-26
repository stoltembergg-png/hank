use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

fn capability_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("capabilities/main.json")
}

#[test]
// @spec:AC-1295
fn desktop_notification_plugin_is_registered_with_minimal_capability() {
    let manifest = fs::read_to_string(manifest_path()).expect("Cargo.toml deve existir");
    let capability: Value = serde_json::from_str(
        &fs::read_to_string(capability_path()).expect("capability deve existir"),
    )
    .expect("capability JSON deve ser válido");
    assert!(manifest.contains("tauri-plugin-notification = \"2.3.3\""));
    let permissions = capability["permissions"]
        .as_array()
        .expect("permissions array");
    let names = permissions
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "notification:allow-is-permission-granted",
            "notification:allow-permission-state",
            "notification:allow-request-permission",
            "notification:allow-notify",
        ]
    );
    assert!(!names.contains(&"notification:default"));
}

#[test]
// @spec:AC-1296
fn tauri_adapter_is_a_real_compiling_module_without_external_panics() {
    let adapter = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/notifications.rs");
    let source = fs::read_to_string(adapter).expect("adapter deve existir");
    assert!(source.contains("pub struct TauriNotificationSink"));
    assert!(source.contains("impl<R: Runtime> NotificationSink for TauriNotificationSink<R>"));
    assert!(!source.contains("unwrap()"));
    assert!(!source.contains("expect("));
}
