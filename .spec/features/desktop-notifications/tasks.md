# Tasks: desktop notifications

> feature: desktop-notifications

## T-1291 — Implementar política bounded de notificação [concluida]
- Refs: US-1284, AC-1285, AC-1286, AC-1287, AC-1288, AC-1295, AC-1296
- Arquivos: `crates/agent-runtime/src/notifications.rs`, `crates/agent-runtime/src/lib.rs`, `crates/agent-runtime/tests/notifications_contract.rs`, `docs/desktop-notifications.md`

## T-1292 — Integrar worker desktop e preferência OS [pendente]
- Refs: US-1284, AC-1287, AC-1295, AC-1296
- Arquivos: `apps/desktop/src-tauri/src/notifications.rs`, `apps/desktop/src-tauri/src/main.rs`, `apps/desktop/src-tauri/tests/notifications_adapter_contract.rs`, `apps/desktop/src-tauri/Cargo.toml`, `apps/desktop/src-tauri/Cargo.lock`, `apps/desktop/src-tauri/capabilities/main.json`, `frontend/src/api/notifications.ts`

A implementação usa o plugin oficial atrás da boundary interna; nenhuma API Tauri entra no runtime.

## T-1294 — Definir boundary interna de capability [concluida]
- Refs: AC-1287
- Arquivos: `crates/agent-runtime/src/notifications.rs`, `crates/agent-runtime/tests/notifications_contract.rs`

Estados de permissão e fallback determinísticos foram implementados sem bloquear o scheduler.

## T-1293 — Auditar e publicar PR-203 [pendente]
- Refs: US-1284, AC-1285, AC-1286, AC-1287, AC-1288, AC-1295, AC-1296
- Arquivos: `.github/workflows/onp-sdd-evidence.yml`, `docs/desktop-notifications.md`
