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

## T-704 — Implementar repository SQLite project-scoped [em-andamento]

- Refs: US-630, AC-735, AC-736
- Arquivos: crates/agent-runtime/src/memory_repo.rs, crates/agent-runtime/src/lib.rs, migrations/0006_memory_storage.sql
- Notas: create/get/list active/archive com queries parametrizadas, foreign key project e optimistic version.

## T-705 — Cobrir migration, isolamento e conflito de versão [em-andamento]

- Refs: US-630, AC-735, AC-736
- Arquivos: crates/agent-runtime/tests/memory_repository_contract.rs
- Notas: CRUD scoped, archived invisível na lista ativa, duplicata e conflito sem mutação.

## T-706 — Documentar repository e registrar verificação [pendente]

- Refs: US-630, AC-735, AC-736
- Arquivos: docs/memory-repository.md, .spec/verification/memory-core.json
- Notas: transação, scope obrigatório, rollback, version conflict e não uso para retrieval.

## T-707 — Implementar taxonomia de memória versionada [em-andamento]

- Refs: US-631, AC-737, AC-738, AC-739, AC-740
- Arquivos: crates/agent-core/src/taxonomy.rs, crates/agent-core/src/lib.rs
- Notas: oito tipos wire-stable, hints explícitos, validação de provenance e rejeição de instrução privilegiada/secret-like.

## T-708 — Cobrir taxonomia e compatibilidade [em-andamento]

- Refs: US-631, AC-737, AC-738, AC-739, AC-740
- Arquivos: crates/agent-core/tests/memory_taxonomy_contract.rs
- Notas: enum/serialization, tipo desconhecido, injection claims, secrets, provenance e versão.

## T-709 — Documentar taxonomy e registrar verificação [pendente]

- Refs: US-631, AC-737, AC-738, AC-739, AC-740
- Arquivos: docs/memory-taxonomy.md, .spec/verification/memory-core.json
- Notas: semântica, retention/importance hints, trust boundary e evolução de versão.

## T-710 — Implementar extractor de candidates data-only [em-andamento]

- Refs: US-632, AC-741, AC-742, AC-743, AC-744
- Arquivos: crates/agent-core/src/memory_candidate.rs, crates/agent-core/src/lib.rs
- Notas: valida project/session/source/taxonomy/confidence, produz Pending e não depende de repository.

## T-711 — Cobrir extractor e rejeições de segurança [em-andamento]

- Refs: US-632, AC-741, AC-742, AC-743, AC-744
- Arquivos: crates/agent-core/tests/memory_candidate_contract.rs
- Notas: identidade/provenance, bounds, injection, secret-like content e ausência de auto-activation.

## T-712 — Documentar pipeline candidate e verificar [pendente]

- Refs: US-632, AC-741, AC-742, AC-743, AC-744
- Arquivos: docs/memory-candidates.md, .spec/verification/memory-core.json
- Notas: conversation→candidate, trust model, review required e rollback.