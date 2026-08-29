# Tasks: automation UI

> feature: automation-ui

## T-1276 — Expor listagem bounded e commands de scheduler [concluida]

- Refs: US-1270, AC-1271, AC-1272, AC-1273
- Arquivos: `crates/agent-runtime/src/scheduler.rs`, `apps/desktop/src-tauri/src/scheduler.rs`, `apps/desktop/src-tauri/src/main.rs`, `frontend/src/api/scheduler.ts`
- Escopo: listar jobs por project com limite/página, criar e atualizar via application boundary; sem executar jobs.

## T-1277 — Construir tela acessível de automations [pendente]

- Refs: US-1270, AC-1272, AC-1274
- Arquivos: `frontend/src/components/AutomationList.tsx`, `frontend/src/api/scheduler.ts`, `frontend/src/components/ProjectDetailView.tsx`
- Escopo: formulário explícito de interval/cron/one-shot, target versionado, timezone, policy, estados e erros.

## T-1278 — Provar contratos e respostas stale/invalidas [pendente]

- Refs: AC-1271, AC-1272, AC-1273, AC-1274
- Arquivos: `crates/agent-runtime/tests/scheduler_job_contract.rs`, `apps/desktop/src-tauri/tests/tauri_ac_tests.rs`, `frontend/tests/automation_ui_ac_tests.test.ts`, `frontend/e2e/desktop-frontend.spec.ts`
- Escopo: testes unitários/IPC/component/E2E sem mock no Desktop E2E; mutação inválida e revisão stale falham fechado.
