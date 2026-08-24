# Tasks: Scoped Skill management UI

> feature: skill-ui

## T-738 — Implementar UI e ponte governada de Skills [pendente]

- Refs: US-641, AC-781, AC-782
- Arquivos: frontend/src/api/skills.ts, frontend/src/components/SkillsPanel.tsx, frontend/src/components/SkillsPanel.css, frontend/src/types/skill.ts, frontend/src/components/ProjectDetailView.tsx, frontend/tests/skills_panel_contract.test.tsx, frontend/e2e/desktop-frontend.spec.ts, docs/skill-ui.md, apps/desktop/src-tauri/src/skills.rs, apps/desktop/src-tauri/src/main.rs, apps/desktop/src-tauri/src/confirmations.rs, apps/desktop/src-tauri/tests/tauri_ac_tests.rs, apps/desktop/src-tauri/Cargo.toml, apps/desktop/src-tauri/Cargo.lock, .github/workflows/onp-sdd-evidence.yml
- Notas: Lista project/global bounded e isolada; bridge Tauri serializa metadados redigidos, deriva capability confiável e delega rollback confirmado ao serviço de binding. Sem editor, execução ou ativação silenciosa.
