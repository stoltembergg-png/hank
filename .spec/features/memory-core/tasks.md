# Tasks: Memory core

> feature: memory-core

## T-701 — Implementar entidade e invariantes bounded [em-andamento]

- Refs: US-629, AC-731, AC-732, AC-733, AC-734
- Arquivos: crates/agent-core/src/memory.rs, crates/agent-core/src/lib.rs
- Notas: entidade project-scoped existente reforçada com validate, limites de conteúdo/confidence, lifecycle versionado e archival/restore.

## T-702 — Cobrir schema e lifecycle com contratos [em-andamento]

- Refs: US-629, AC-731, AC-732, AC-733, AC-734
- Arquivos: crates/agent-core/tests/memory_entity_contract.rs
- Notas: candidate inicial, negações bounded, transições, versões e conteúdo arquivado.

## T-703 — Documentar boundary de memória e verificar [pendente]

- Refs: US-629, AC-731, AC-732, AC-733, AC-734
- Arquivos: docs/memory-core.md, .spec/verification/memory-core.json
- Notas: conteúdo não confiável, provenance, approval, isolamento, lifecycle e ausência de repository/retrieval.
