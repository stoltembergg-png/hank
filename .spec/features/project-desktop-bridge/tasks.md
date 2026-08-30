# Tasks: Desktop project lifecycle bridge

> feature: project-desktop-bridge

## T-116 — Corrigir bridge Tauri e contratos Project [concluida]

- Refs: US-110, AC-111, AC-112, AC-113, AC-114
- Arquivos: `apps/desktop/src-tauri/src/projects.rs`, `apps/desktop/src-tauri/src/main.rs`, `apps/desktop/src-tauri/src/confirmations.rs`, `apps/desktop/src-tauri/tests/tauri_ac_tests.rs`, `crates/agent-runtime/src/project_repo.rs`, `frontend/src/api/projects.ts`, `frontend/tests/create_project_ac_tests.test.ts`, `frontend/tests/project_list_ac_tests.test.ts`
- Notas: commands usam services reais e o pool SQLite migrado do boot; fallback sintético foi removido.

## T-1388 — Expor lifecycle project-scoped de Sessions no desktop [em-andamento]

- Refs: US-110, AC-111, AC-112, AC-113, AC-114
- Arquivos: `apps/desktop/src-tauri/src/sessions.rs`, `apps/desktop/src-tauri/src/main.rs`, `apps/desktop/src-tauri/src/confirmations.rs`, `apps/desktop/src-tauri/tests/tauri_ac_tests.rs`, `crates/agent-runtime/src/session_repo.rs`, `crates/agent-runtime/src/session_service.rs`, `crates/agent-runtime/tests/session_service_contract.rs`, `frontend/src/api/sessions.ts`, `frontend/src/types/session.ts`, `frontend/src/components/AgentList.tsx`, `frontend/src/components/AgentList.css`, `frontend/src/components/SessionList.tsx`, `frontend/src/components/SessionList.css`, `frontend/src/components/ProjectDetailView.tsx`, `frontend/tests/session_bridge_contract.test.ts`, `frontend/tests/session_workbench.test.tsx`, `desktop-e2e/specs/project-lifecycle.e2e.mjs`, `docs/desktop-e2e-project-lifecycle.md`
- Notas: commands de criação/listagem usam o pool SQLite do boot, escopam por `project_id`/`agent_id`, retornam somente metadados bounded e preservam a separação entre lifecycle persistente e execução de provider.
