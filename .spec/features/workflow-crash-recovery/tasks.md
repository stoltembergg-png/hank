# Tasks: workflow crash recovery

> feature: workflow-crash-recovery

## T-1059 — Implementar lease, fencing e scanner bounded [concluida]

- Refs: US-1050, AC-1051, AC-1052, AC-1053
- Arquivos: `migrations/0015_workflow_recovery.sql`, `crates/agent-runtime/src/workflow_recovery.rs`, `crates/agent-runtime/tests/workflow_recovery_contract.rs`
- Escopo: lease expiry, epoch fencing, unknown quarantine, bounded recovery report e redacted diagnostics.
