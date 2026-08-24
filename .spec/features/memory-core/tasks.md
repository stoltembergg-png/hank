# Tasks: Memory core

> feature: memory-core

## T-701 — Implementar entidade e invariantes bounded [concluida]

- Refs: US-629, AC-731, AC-732, AC-733, AC-734
- Arquivos: crates/agent-core/src/memory.rs, crates/agent-core/src/lib.rs
- Notas: entidade project-scoped existente reforçada com validate, limites de conteúdo/confidence, lifecycle versionado e archival/restore.

## T-702 — Cobrir schema e lifecycle com contratos [concluida]

- Refs: US-629, AC-731, AC-732, AC-733, AC-734
- Arquivos: crates/agent-core/tests/memory_entity_contract.rs
- Notas: candidate inicial, negações bounded, transições, versões e conteúdo arquivado.

## T-703 — Documentar boundary de memória e verificar [concluida]

- Refs: US-629, AC-731, AC-732, AC-733, AC-734
- Arquivos: docs/memory-core.md, .spec/verification/memory-core.json
- Notas: conteúdo não confiável, provenance, approval, isolamento, lifecycle e ausência de repository/retrieval; `onp-spec verify memory-core` registrou prova PASS para AC-731..AC-776, incluindo o slice AC-731..AC-734 deste incremento.

## T-704 — Implementar repository SQLite project-scoped [concluida]

- Refs: US-630, AC-735, AC-736
- Arquivos: crates/agent-runtime/src/memory_repo.rs, crates/agent-runtime/src/lib.rs, migrations/0006_memory_storage.sql
- Notas: create/get/list active/archive com queries parametrizadas, foreign key project e optimistic version.

## T-705 — Cobrir migration, isolamento e conflito de versão [concluida]

- Refs: US-630, AC-735, AC-736
- Arquivos: crates/agent-runtime/tests/memory_repository_contract.rs
- Notas: CRUD scoped, archived invisível na lista ativa, duplicata e conflito sem mutação.

## T-706 — Documentar repository e registrar verificação [concluida]

- Refs: US-630, AC-735, AC-736
- Arquivos: docs/memory-repository.md, .spec/verification/memory-core.json
- Notas: transação, scope obrigatório, rollback, version conflict e não uso para retrieval; artifact ONP `memory-core.json` registra PASS para AC-735..AC-736.

## T-707 — Implementar taxonomia de memória versionada [concluida]

- Refs: US-631, AC-737, AC-738, AC-739, AC-740
- Arquivos: crates/agent-core/src/taxonomy.rs, crates/agent-core/src/lib.rs
- Notas: oito tipos wire-stable, hints explícitos, validação de provenance e rejeição de instrução privilegiada/secret-like; contrato Rust passou com 4/4 testes.

## T-708 — Cobrir taxonomia e compatibilidade [concluida]

- Refs: US-631, AC-737, AC-738, AC-739, AC-740
- Arquivos: crates/agent-core/tests/memory_taxonomy_contract.rs
- Notas: enum/serialization, tipo desconhecido, injection claims, secrets, provenance e versão; contrato Rust passou com 4/4 testes.

## T-709 — Documentar taxonomy e registrar verificação [concluida]

- Refs: US-631, AC-737, AC-738, AC-739, AC-740
- Arquivos: docs/memory-taxonomy.md, .spec/verification/memory-core.json
- Notas: semântica, retention/importance hints, trust boundary e evolução de versão; documentação existente e artifact ONP `memory-core.json` registram PASS para AC-737..AC-740.

## T-710 — Implementar extractor de candidates data-only [concluida]

- Refs: US-632, AC-741, AC-742, AC-743, AC-744
- Arquivos: crates/agent-core/src/memory_candidate.rs, crates/agent-core/src/lib.rs
- Notas: valida project/session/source/taxonomy/confidence, produz Pending e não depende de repository; contrato Rust passou com 4/4 testes.

## T-711 — Cobrir extractor e rejeições de segurança [concluida]

- Refs: US-632, AC-741, AC-742, AC-743, AC-744
- Arquivos: crates/agent-core/tests/memory_candidate_contract.rs
- Notas: identidade/provenance, bounds, injection, secret-like content e ausência de auto-activation; contrato Rust passou com 4/4 testes.

## T-712 — Documentar pipeline candidate e verificar [concluida]

- Refs: US-632, AC-741, AC-742, AC-743, AC-744
- Arquivos: docs/memory-candidates.md, .spec/verification/memory-core.json
- Notas: conversation→candidate, trust model, review required e rollback; documentação existente e artifact ONP `memory-core.json` registram PASS para AC-741..AC-744.

## T-713 — Implementar scorer de importance versionado [concluida]

- Refs: US-633, AC-745, AC-746, AC-747, AC-748
- Arquivos: crates/agent-core/src/importance.rs, crates/agent-core/src/lib.rs
- Notas: score determinístico por confidence/recency/repetition/type/source, threshold bounded, policy/trace e content excluído; contrato Rust passou com 4/4 testes.

## T-714 — Cobrir scoring e manipulação de conteúdo [concluida]

- Refs: US-633, AC-745, AC-746, AC-747, AC-748
- Arquivos: crates/agent-core/tests/memory_importance_contract.rs
- Notas: determinismo, threshold, injection/secret, policy inválida e identity; contrato Rust passou com 4/4 testes.

## T-715 — Documentar importance e registrar verificação [concluida]

- Refs: US-633, AC-745, AC-746, AC-747, AC-748
- Arquivos: docs/memory-importance.md, .spec/verification/memory-core.json
- Notas: fórmula, factors, thresholds, rollback e papel não autoritativo do modelo; documentação existente e artifact ONP `memory-core.json` registram PASS para AC-745..AC-748.

## T-716 — Implementar dedupe scoped e determinístico [concluida]

- Refs: US-634, AC-749, AC-750, AC-751, AC-752
- Arquivos: crates/agent-core/src/dedupe.rs, crates/agent-core/src/lib.rs
- Notas: normalização bounded, duplicate/conflict/new por projeto/agente/tipo/chave, sem similaridade vetorial; contrato Rust passou com 4/4 testes.

## T-717 — Cobrir dedupe, conflito e rollback [concluida]

- Refs: US-634, AC-749, AC-750, AC-751, AC-752
- Arquivos: crates/agent-core/tests/memory_dedupe_contract.rs
- Notas: exact match, cross-project isolation, conflict reviewable, retry idempotente e rollback; contrato Rust passou com 4/4 testes.

## T-718 — Documentar dedupe e registrar verificação [concluida]

- Refs: US-634, AC-749, AC-750, AC-751, AC-752
- Arquivos: docs/memory-dedupe.md, .spec/verification/memory-core.json
- Notas: equivalência, conflito, provenance, limites e rollback; documentação existente e artifact ONP `memory-core.json` registram PASS para AC-749..AC-752.

## T-719 — Implementar keyword retrieval bounded [concluida]

- Refs: US-635, AC-753, AC-754, AC-755, AC-756
- Arquivos: crates/agent-core/src/keyword.rs, crates/agent-core/src/lib.rs
- Notas: tokens seguros, filtros project/agent/status, ranking deterministic, result/byte bounds e trace obrigatório; contrato Rust passou com 4/4 testes.

## T-720 — Cobrir retrieval e isolamento [concluida]

- Refs: US-635, AC-753, AC-754, AC-755, AC-756
- Arquivos: crates/agent-core/tests/memory_keyword_contract.rs
- Notas: scope/status, terms oversized, duplicate identity, byte budget, trace e ranking; contrato Rust passou com 4/4 testes.

## T-721 — Documentar keyword retrieval e verificar [concluida]

- Refs: US-635, AC-753, AC-754, AC-755, AC-756
- Arquivos: docs/memory-keyword-retrieval.md, .spec/verification/memory-core.json
- Notas: query contract, ranking, bounds, trust/provenance e rollback; documentação existente e artifact ONP `memory-core.json` registram PASS para AC-753..AC-756.

## T-722 — Implementar interface provider-agnostic de embeddings [concluida]

- Refs: US-636, AC-757, AC-758, AC-759, AC-760
- Arquivos: crates/agent-core/src/embedding.rs, crates/agent-core/src/lib.rs
- Notas: request/response dimensioned, model/version/trace, batch/reference/budget/cancel bounds e mock offline determinístico; implementação existente validada pelo contrato Rust com 4/4 testes.

## T-723 — Cobrir contrato e mock de embeddings [concluida]

- Refs: US-636, AC-757, AC-758, AC-759, AC-760
- Arquivos: crates/agent-core/tests/memory_embedding_contract.rs
- Notas: dimensão, identity, invalid model/dimension/batch, budget, cancellation e references sem raw content; contrato Rust passou com 4/4 testes.

## T-724 — Documentar embeddings e registrar verificação [concluida]

- Refs: US-636, AC-757, AC-758, AC-759, AC-760
- Arquivos: docs/memory-embeddings.md, .spec/verification/memory-core.json
- Notas: interface, cost policy, privacy, mock e ausência de backend/provider concreto; documentação existente e artifact ONP `memory-core.json` registram PASS para AC-757..AC-760.

## T-725 — Implementar backend vetorial local e opcional [concluida]

- Refs: US-637, AC-761, AC-762, AC-763, AC-764
- Arquivos: crates/agent-core/src/vector.rs, crates/agent-core/src/lib.rs
- Notas: upsert idempotente, cosine ranking, project/agent/model scope, archive, k/bytes bounds e rebuild atomic rollback; implementação existente validada pelo contrato Rust com 4/4 testes.

## T-726 — Cobrir vector index e rollback [concluida]

- Refs: US-637, AC-761, AC-762, AC-763, AC-764
- Arquivos: crates/agent-core/tests/memory_vector_contract.rs
- Notas: isolation, dimension/model, upsert/archive, limits e rebuild failure; contrato Rust passou com 4/4 testes.

## T-727 — Documentar vector backend e registrar verificação [concluida]

- Refs: US-637, AC-761, AC-762, AC-763, AC-764
- Arquivos: docs/memory-vector-retrieval.md, .spec/verification/memory-core.json
- Notas: lifecycle, privacy, rollback, limits e independência de provider remoto; documentação existente e artifact ONP `memory-core.json` registram PASS para AC-761..AC-764.

## T-728 — Implementar selector de memória policy-first e bounded [concluida]

- Refs: US-638, AC-765, AC-766, AC-767, AC-768
- Arquivos: crates/agent-runtime/src/context/memory_selector.rs, crates/agent-runtime/src/context.rs
- Notas: selector read-only recebe candidates carregados, filtra project/agent/status/policy/capability antes do ranking, mantém memória untrusted, aplica budget/dedupe e não acessa SQLite/UI/provider; implementação existente validada pelo contrato Rust com 4/4 testes.

## T-729 — Cobrir seleção, isolamento, budget e rejeições [concluida]

- Refs: US-638, AC-765, AC-766, AC-767, AC-768
- Arquivos: crates/agent-runtime/tests/memory_selector_contract.rs
- Notas: project/agent scope, archived/policy denial, deterministic ranking, duplicate key, token budget, injection/secret-like content, empty path, trace e cancellation; contrato Rust passou com 4/4 testes.

## T-730 — Documentar selector e registrar verificação [concluida]

- Refs: US-638, AC-765, AC-766, AC-767, AC-768
- Arquivos: docs/memory-selector.md, .spec/verification/memory-core.json
- Notas: filtros antes do ranking, trust/provenance, ordering, budgets, omission reasons, failure behavior e ausência de write/activation; documentação existente e artifact ONP `memory-core.json` registram PASS para AC-765..AC-768.

## T-731 — Implementar contrato e bridge project-scoped de memória [concluida]

- Refs: US-639, AC-769, AC-772
- Arquivos: frontend/src/types/memory.ts, frontend/src/api/memory.ts
- Notas: request exige project_id, bridge Tauri é opcional, fallback browser é vazio e não existe acesso direto a SQLite/localStorage; contrato frontend validado com 4/4 testes.

## T-732 — Implementar painel read-only de memória no detalhe do projeto [concluida]

- Refs: US-639, AC-769, AC-770, AC-771, AC-772
- Arquivos: frontend/src/components/MemoryPanel.tsx, frontend/src/components/MemoryPanel.css, frontend/src/components/ProjectDetailView.tsx
- Notas: cards project-scoped, lifecycle/provenance/scores/trace, filtros status/type, redaction/truncation, loading/error/empty e nenhuma edição/activation; contrato frontend validado com 4/4 testes.

## T-733 — Cobrir UI de memória e registrar verificação [concluida]

- Refs: US-639, AC-769, AC-770, AC-771, AC-772
- Arquivos: frontend/tests/memory_panel_contract.test.tsx, docs/memory-ui.md, .spec/verification/memory-core.json
- Notas: bridge request, foreign-project filtering, escaping, secret redaction, bounded preview, status/provenance, filters, accessibility e E2E aplicável; documentação existente e artifact ONP `memory-core.json` registram PASS para AC-769..AC-772.

## T-734 — Implementar serviço explícito de mutations project/version-scoped [concluida]

- Refs: US-640, AC-773, AC-774, AC-775, AC-776
- Arquivos: crates/agent-runtime/src/memory_service.rs, crates/agent-runtime/src/memory_repo.rs, crates/agent-runtime/src/lib.rs
- Notas: contexto actor/project/trace/capability/policy/operation, update/approve/reject/archive/restore, optimistic version e repository update parametrizado. Implementação existente validada pelo contrato Rust com 4/4 testes.

## T-735 — Cobrir mutations, lifecycle e rejeições fail-closed [concluida]

- Refs: US-640, AC-773, AC-774, AC-775, AC-776
- Arquivos: crates/agent-runtime/tests/memory_edit_service_contract.rs
- Notas: contexto válido, lifecycle reversível, foreign scope, capability/policy, oversized/stale version e duplicate operation sem replay. Contrato Rust passou com 4/4 testes.

## T-736 — Expor mutation por Application/Tauri e UI com confirmação [concluida]

- Refs: US-640, AC-773, AC-774, AC-775, AC-776
- Arquivos: apps/desktop/src-tauri/src/memory.rs, apps/desktop/src-tauri/src/main.rs, frontend/src/api/memory.ts, frontend/src/components/MemoryPanel.tsx
- Notas: command boundary somente após actor/confirmation/trace, sem SQLite no frontend, confirmação explícita, audit metadata e rollback/version conflict visíveis. Bridge Tauri e controles UI validados por 7 testes frontend, 9 unitários desktop e 6 contratos Tauri.

## T-737 — Documentar edição, aprovação e rollback e registrar verificação [concluida]

- Refs: US-640, AC-773, AC-774, AC-775, AC-776
- Arquivos: docs/memory-editing.md, .spec/verification/memory-core.json
- Notas: workflow editorial, policy/capability, optimistic version, replay prevention, audit e privacy documentados em `docs/memory-editing.md`; `onp-spec verify memory-core` registra PASS para AC-773..AC-776 após a integração do bridge Tauri/UI (PR #210, SHA `2331bd5`).

## T-738 — Tornar mutações de índice explicitamente project-scoped [concluida]

- Refs: US-641, AC-779
- Arquivos: crates/agent-core/src/vector.rs, crates/agent-core/tests/memory_vector_contract.rs
- Notas: archive exige `ProjectId` e rejeita tentativa foreign sem desativar o registro correto. Contrato vector passou com 4/4 testes.

## T-739 — Verificar isolamento cross-project em todas as boundaries [concluida]

- Refs: US-641, AC-780
- Arquivos: crates/agent-runtime/tests/memory_isolation_contract.rs, docs/memory-isolation.md, .spec/verification/memory-core.json
- Notas: repository, selector e mutation service cobertos com dois projetos; vector index coberto por AC-779; bridge Tauri mantém project_id obrigatório e SQLite atrás do estado gerenciado.

## T-740 — Implementar modelo e resolver de policy project/agent-scoped [em-andamento]

- Refs: US-642, AC-781, AC-782, AC-783, AC-784
- Arquivos: crates/agent-core/src/memory_policy.rs, crates/agent-core/src/memory.rs, crates/agent-core/src/lib.rs
- Notas: política versionada, bounded, deny-default, identidade project/agent, precedência system/security/project/agent e budgets de tipo/token/custo; contratos Rust passam com 4/4 testes.

## T-741 — Persistir policy e aplicar em todos os caminhos de memória [pendente]

- Refs: US-642, AC-781, AC-782, AC-783, AC-784
- Arquivos: migrations/*memory_policy*, crates/agent-runtime/src/memory_policy_repo.rs, memory_service.rs, context/memory_selector.rs
- Notas: nenhum read/write/learn pode ignorar policy; atualização deve ser optimistic/versioned, auditável e rollback-safe.

## T-742 — Documentar precedência, autonomia, approval e rollback [pendente]

- Refs: US-642, AC-781, AC-782, AC-783, AC-784
- Arquivos: docs/memory-policy.md, .spec/verification/memory-core.json
- Notas: documentar decisão sem conteúdo raw, capability, actor/project/agent/trace, budgets, retenção e interação com autonomy.
