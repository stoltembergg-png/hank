# Tasks: workflow crash recovery

> feature: workflow-crash-recovery

## T-1059 — Implementar lease, fencing e scanner bounded [concluida]

- Refs: US-1050, AC-1051, AC-1052, AC-1053
- Arquivos: `migrations/0015_workflow_recovery.sql`, `crates/agent-runtime/src/workflow_recovery.rs`, `crates/agent-runtime/tests/workflow_recovery_contract.rs`, `tools/security/workflow-crash-recovery-feature-tests.mjs`, `docs/workflow-recovery-tests.md`, `.github/workflows/ci-workflow-recovery.yml`, `onpspec.config.json`, `.github/workflows/onp-sdd-evidence.yml`
- Escopo: lease expiry, epoch fencing, unknown quarantine, bounded recovery report e redacted diagnostics.
