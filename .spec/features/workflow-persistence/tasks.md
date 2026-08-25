# Tasks: Workflow persistence

> feature: workflow-persistence

## T-963 — Persistir workflow, nodes e edges [concluida]

- Refs: US-959, AC-960, AC-961, AC-962
- Arquivos: `migrations/0013_workflow_storage.sql`, `crates/agent-runtime/src/workflow_repo.rs`, `crates/agent-runtime/tests/workflow_persistence_contract.rs`
- Notas: migration forward-only; writes use one transaction; reads are filtered by project_id; não há executor ou scheduler.
