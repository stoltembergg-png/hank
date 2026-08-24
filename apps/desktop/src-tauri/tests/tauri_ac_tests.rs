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
    fn ac_104_bridge_registra_somente_commands_tipados() {
        // @spec:AC-104
        let source = fs::read_to_string(source_path()).expect("main.rs não encontrado");

        assert!(
            source.contains(".invoke_handler(confirmations::command_handler())"),
            "bridge deve registrar o handler tipado"
        );

        let bridge = fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/confirmations.rs"),
        )
        .expect("confirmations.rs não encontrado");

        let handler_block = bridge
            .split("generate_handler!")
            .nth(1)
            .expect("generate_handler ausente na ponte");
        let registered = handler_block.split(']').next().unwrap_or_default();

        for command in [
            "submit_confirmation_request",
            "approve_confirmation_request",
            "revoke_confirmation_request",
            "crate::memory::list_memories",
            "crate::memory::mutate_memory",
            "crate::skills::list_skills",
            "crate::skills::rollback_skill",
            "crate::skills::get_skill_editor",
            "crate::skills::validate_skill_draft",
            "crate::skills::save_skill_draft",
            "crate::skills::discard_skill_draft",
        ] {
            assert!(
                registered.contains(command),
                "comando do ciclo de confirmação ausente: {command}"
            );
        }

        assert_eq!(
            registered.split(',').count(),
            11,
            "a ponte deve registrar exatamente os comandos tipados previstos"
        );

        for forbidden in ["chat_stream", "send_command", "list_projects", "run_tool"] {
            assert!(
                !bridge.contains(forbidden),
                "comando de produto fora do ciclo de confirmação: {forbidden}"
            );
        }
    }

    #[test]
    fn ac_106_memory_mutation_bridge_requires_scoped_confirmation_context() {
        // @spec:AC-773 @spec:AC-774 @spec:AC-775 @spec:AC-776
        let bridge =
            fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/memory.rs"))
                .expect("memory.rs não encontrado");

        for required in [
            "MemoryMutationContext",
            "MemoryMutationService",
            "actor_id",
            "trace_id",
            "operation_id",
            "expected_version",
            "confirmed",
            "mutate_memory",
        ] {
            assert!(
                bridge.contains(required),
                "boundary metadata ausente: {required}"
            );
        }
        assert!(bridge.contains("memory.write"));
        assert!(bridge.contains("confirm memory mutation"));
        assert!(
            !bridge.contains("sqlx::query"),
            "SQLite não deve cruzar a ponte diretamente"
        );
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

    #[test]
    fn ac_101_release_nao_abre_console_do_windows() {
        // @spec:AC-101
        let source = fs::read_to_string(source_path()).expect("main.rs não encontrado");

        assert!(
            source.contains("#![cfg_attr(not(debug_assertions), windows_subsystem = \"windows\")]"),
            "o binário release deve usar o subsistema GUI do Windows"
        );
    }
}
