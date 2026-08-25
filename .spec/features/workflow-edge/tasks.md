# Tasks: Workflow edge and DAG validation

> feature: workflow-edge

## T-957 — Implementar edges e validação acíclica [concluida]

- Refs: US-953, AC-954, AC-955, AC-956
- Arquivos: `crates/workflow-core/src/lib.rs`, `crates/workflow-core/tests/edge_contract.rs`
- Notas: validação é bounded e declarativa; labels de condição não são interpretadas.
