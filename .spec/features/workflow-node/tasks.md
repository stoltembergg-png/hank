# Tasks: Workflow node schema

> feature: workflow-node

## T-952 — Implementar schema versionado de workflow nodes [concluida]

- Refs: US-948, AC-949, AC-950, AC-951
- Arquivos: `crates/workflow-core/src/lib.rs`, `crates/workflow-core/tests/node_contract.rs`
- Notas: tipos e validações são declarativos; nenhum handler, scheduler ou storage é chamado.
