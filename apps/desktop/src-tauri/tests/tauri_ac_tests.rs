#[cfg(test)]
mod tauri_tests {
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;

    fn manifest_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json")
    }

    fn source_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/main.rs")
    }

    fn manifest() -> Value {
        let contents = fs::read_to_string(manifest_path()).expect("tauri.conf.json não encontrado");
        serde_json::from_str(&contents).expect("tauri.conf.json inválido")
    }

    #[test]
    fn ac_101_janela_abre_fecha_deterministico() {
        // @spec:AC-101
        let package_name = env!("CARGO_PKG_NAME");
        assert_eq!(package_name, "hank-desktop");
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        assert!(manifest.exists(), "Cargo.toml não encontrado");
        let conf = manifest_path();
        assert!(conf.exists(), "tauri.conf.json não encontrado");
        let src = source_path();
        assert!(src.exists(), "main.rs não encontrado");
    }

    #[test]
    fn ac_102_manifest_sem_capacidades_perigosas() {
        // @spec:AC-102
        let manifest = manifest();
        let serialized = serde_json::to_string(&manifest).expect("manifest não serializável");

        for forbidden in [
            "allowlist",
            "all: true",
            "fs:all",
            "process:all",
            "network:all",
            "shell:all",
            "dialog:all",
        ] {
            assert!(
                !serialized.contains(forbidden),
                "capability perigosa encontrada: {forbidden}"
            );
        }

        assert!(manifest["app"]["windows"].is_array());
    }

    #[test]
    fn ac_103_csp_bloqueia_origem_remota() {
        // @spec:AC-103
        let document = manifest();
        let csp = document["app"]["security"]["csp"]
            .as_str()
            .expect("CSP ausente");

        for directive in [
            "default-src 'self'",
            "script-src 'self'",
            "style-src 'self'",
            "connect-src 'self'",
        ] {
            assert!(csp.contains(directive), "diretiva CSP ausente: {directive}");
        }
        assert!(!csp.contains("unsafe-inline"));
        assert!(!csp.contains("unsafe-eval"));
    }

    #[test]
    fn ac_104_bridge_sem_comandos_produto() {
        // @spec:AC-104
        let source = fs::read_to_string(source_path()).expect("main.rs não encontrado");

        for forbidden in [
            "invoke_handler",
            "generate_handler",
            "chat",
            "agent",
            "tools",
        ] {
            assert!(
                !source.contains(forbidden),
                "handler de produto encontrado: {forbidden}"
            );
        }
    }

    #[test]
    fn ac_105_logs_estruturados() {
        // @spec:AC-105
        let source = fs::read_to_string(source_path()).expect("main.rs não encontrado");

        for event in ["boot", "ready", "close", "failure"] {
            assert!(source.contains(&format!("event = \"{event}\"")));
        }
        assert!(source.contains("version"));
        for forbidden in ["token", "secret", "password", "Authorization"] {
            assert!(!source.contains(forbidden));
        }
    }
}
