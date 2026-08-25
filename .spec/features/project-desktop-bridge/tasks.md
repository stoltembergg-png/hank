# Tasks: Desktop project lifecycle bridge

> feature: project-desktop-bridge

## T-116 — Corrigir bridge Tauri e contratos Project [concluida]

- Refs: US-110, AC-111, AC-112, AC-113, AC-114
- Arquivos: `apps/desktop/src-tauri/src/projects.rs`, `apps/desktop/src-tauri/src/main.rs`, `apps/desktop/src-tauri/src/confirmations.rs`, `apps/desktop/src-tauri/tests/tauri_ac_tests.rs`, `crates/agent-runtime/src/project_repo.rs`, `frontend/src/api/projects.ts`, `frontend/tests/create_project_ac_tests.test.ts`, `frontend/tests/project_list_ac_tests.test.ts`
- Notas: commands usam services reais e o pool SQLite migrado do boot; fallback sintético foi removido.
