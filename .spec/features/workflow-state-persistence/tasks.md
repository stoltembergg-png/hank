# Tasks: workflow state persistence

> feature: workflow-state-persistence

## T-1041 — Adicionar schema e repository transacional de state [concluida]

- Refs: US-1032, AC-1033, AC-1034, AC-1035
- Arquivos: `migrations/0014_workflow_state.sql`, `crates/agent-runtime/src/workflow_state_repo.rs`, `crates/agent-runtime/tests/workflow_state_contract.rs`
- Escopo: run/node tables, transition journal, generation/CAS, idempotency, bounded checkpoints e pending approval/delay anchors.
