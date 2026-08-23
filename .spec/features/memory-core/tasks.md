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

## T-713 — Implementar scorer de importance versionado [em-andamento]

- Refs: US-633, AC-745, AC-746, AC-747, AC-748
- Arquivos: crates/agent-core/src/importance.rs, crates/agent-core/src/lib.rs
- Notas: score determinístico por confidence/recency/repetition/type/source, threshold bounded, policy/trace e content excluído.

## T-714 — Cobrir scoring e manipulação de conteúdo [em-andamento]

- Refs: US-633, AC-745, AC-746, AC-747, AC-748
- Arquivos: crates/agent-core/tests/memory_importance_contract.rs
- Notas: determinismo, threshold, injection/secret, policy inválida e identity.

## T-715 — Documentar importance e registrar verificação [pendente]

- Refs: US-633, AC-745, AC-746, AC-747, AC-748
- Arquivos: docs/memory-importance.md, .spec/verification/memory-core.json
- Notas: fórmula, factors, thresholds, rollback e papel não autoritativo do modelo.

## T-716 — Implementar dedupe scoped e determinístico [em-andamento]

- Refs: US-634, AC-749, AC-750, AC-751, AC-752
- Arquivos: crates/agent-core/src/dedupe.rs, crates/agent-core/src/lib.rs
- Notas: normalização bounded, duplicate/conflict/new por projeto/agente/tipo/chave, sem similaridade vetorial.

## T-717 — Cobrir dedupe, conflito e rollback [em-andamento]

- Refs: US-634, AC-749, AC-750, AC-751, AC-752
- Arquivos: crates/agent-core/tests/memory_dedupe_contract.rs
- Notas: exact match, cross-project isolation, conflict reviewable, retry idempotente e rollback.

## T-718 — Documentar dedupe e registrar verificação [pendente]

- Refs: US-634, AC-749, AC-750, AC-751, AC-752
- Arquivos: docs/memory-dedupe.md, .spec/verification/memory-core.json
- Notas: equivalência, conflito, provenance, limites e rollback.

## T-719 — Implementar keyword retrieval bounded [em-andamento]

- Refs: US-635, AC-753, AC-754, AC-755, AC-756
- Arquivos: crates/agent-core/src/keyword.rs, crates/agent-core/src/lib.rs
- Notas: tokens seguros, filtros project/agent/status, ranking deterministic, result/byte bounds e trace obrigatório.

## T-720 — Cobrir retrieval e isolamento [em-andamento]

- Refs: US-635, AC-753, AC-754, AC-755, AC-756
- Arquivos: crates/agent-core/tests/memory_keyword_contract.rs
- Notas: scope/status, terms oversized, duplicate identity, byte budget, trace e ranking.

## T-721 — Documentar keyword retrieval e verificar [pendente]

- Refs: US-635, AC-753, AC-754, AC-755, AC-756
- Arquivos: docs/memory-keyword-retrieval.md, .spec/verification/memory-core.json
- Notas: query contract, ranking, bounds, trust/provenance e rollback.

## T-722 — Implementar interface provider-agnostic de embeddings [em-andamento]

- Refs: US-636, AC-757, AC-758, AC-759, AC-760
- Arquivos: crates/agent-core/src/embedding.rs, crates/agent-core/src/lib.rs
- Notas: request/response dimensioned, model/version/trace, batch/reference/budget/cancel bounds e mock offline determinístico.

## T-723 — Cobrir contrato e mock de embeddings [em-andamento]

- Refs: US-636, AC-757, AC-758, AC-759, AC-760
- Arquivos: crates/agent-core/tests/memory_embedding_contract.rs
- Notas: dimensão, identity, invalid model/dimension/batch, budget, cancellation e references sem raw content.

## T-724 — Documentar embeddings e registrar verificação [pendente]

- Refs: US-636, AC-757, AC-758, AC-759, AC-760
- Arquivos: docs/memory-embeddings.md, .spec/verification/memory-core.json
- Notas: interface, cost policy, privacy, mock e ausência de backend/provider concreto.