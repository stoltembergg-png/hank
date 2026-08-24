# Tasks: Governed Skill draft editor

> feature: skill-editor

## T-744 — Implementar editor de rascunho governado [concluida]

- Refs: US-644, AC-787, AC-788, AC-789, AC-790
- Arquivos: `frontend/src/api/skillEditor.ts`, `frontend/src/types/skillEditor.ts`, `frontend/src/components/SkillEditor.tsx`, `frontend/src/components/SkillsPanel.tsx`, `frontend/src/components/ProjectDetailView.tsx`, `frontend/tests/skill_editor_contract.test.tsx`, `crates/agent-runtime/src/skill_editor.rs`, `crates/agent-runtime/src/skill_repo.rs`, `crates/agent-runtime/tests/skill_editor_contract.rs`, `apps/desktop/src-tauri/src/skills.rs`, `apps/desktop/src-tauri/src/confirmations.rs`, `apps/desktop/src-tauri/tests/tauri_ac_tests.rs`, `.github/workflows/onp-sdd-evidence.yml`, `docs/skill-editor.md`
- Notas: editor sem autosave, ponte com quatro comandos tipados, validação pelo parser, Draft imutável deduplicado, descarte explícito, isolamento de projeto e nenhum comando de ativação/execução.
