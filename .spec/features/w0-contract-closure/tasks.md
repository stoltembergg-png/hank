# Tasks: W0 contract closure

> feature: w0-contract-closure

## T-001 — Consolidar baseline normativo W0 [concluida]
- Refs: US-001, US-002, US-003, US-004, US-005, AC-001, AC-004, AC-007, AC-010, AC-016
- Arquivos: .planning/master/blocker-closure-plan.md, .planning/master/sdd-master.md, .planning/master/architecture-invariants.md, .planning/master/dependency-dag.md, .planning/master/PR-EXECUTION-CONTRACT.md, .planning/master/agent-development-policy.md, .spec/constituicao.md, onpspec.config.json, .spec/features/w0-contract-closure/spec.md, .spec/features/w0-contract-closure/tasks.md
- Notas: Registrar SHA/tree/policy alvo, inventário dos cinco blockers e divergências atuais. Depende de nenhuma tarefa.

## T-002 — Definir contrato de fronteiras e ownership [concluida]
- Refs: US-001, US-002, AC-001, AC-002, AC-003, AC-004, AC-005, AC-006
- Arquivos: .planning/contracts/ADR-AB-001.md, .planning/contracts/AB-001-layer-ownership.md, .planning/contracts/architecture-graph.schema.json, .planning/contracts/architecture-graph.json, .planning/contracts/architecture-graph.invalid-fixtures.json
- Notas: Derivar o contrato dos blockers ARCH-001/ARCH-002; não criar código de produto. Depende de T-001.

## T-003 — Definir schema e validator da fila/DAG [concluida]
- Refs: US-003, AC-007, AC-008, AC-009
- Arquivos: .planning/contracts/queue-card.schema.json, .planning/contracts/queue-validator-contract.md, .planning/contracts/queue-invalid-fixtures.json, .planning/master/queue-index.md, .planning/master/dependency-dag.md
- Notas: Cobrir 270 cards, labels, dependências, M16 e PR-001. Depende de T-001.

## T-004 — Definir PR Execution Contract e evidence manifest [concluida]
- Refs: US-004, AC-010, AC-011, AC-012, AC-013
- Arquivos: .planning/contracts/PR-EXECUTION-CONTRACT.schema.json, .planning/contracts/evidence-manifest.schema.json, .planning/contracts/execution-gate-contract.md, .planning/contracts/execution-invalid-fixtures.json
- Notas: Incluir preflight, branch/worktree, path allowlist, command policy, reviewer identity, SHA/tree/policy invalidation e rollback. Depende de T-001.

## T-005 — Definir gate negativo e matriz de fechamento [concluida]
- Refs: US-005, AC-014, AC-015, AC-016
- Arquivos: .planning/contracts/w0-negative-test-matrix.json, .planning/contracts/w0-closure-gate.md, .planning/master/blocker-closure-plan.md
- Notas: Mapear cada fixture para blocker, invariantes e condição de desbloqueio. Depende de T-002, T-003 e T-004.

## T-006 — Auditar especificação ONP e evidência W0 [concluida]
- Refs: US-001, US-002, US-003, US-004, US-005, AC-001, AC-003, AC-005, AC-008, AC-011, AC-013, AC-014, AC-015, AC-016
- Arquivos: .spec/features/w0-contract-closure/spec.md, .spec/features/w0-contract-closure/tasks.md, test/w0-contract-closure.spec.test.js, tools/w0-contract-validator.mjs, .github/workflows/w0-contract-gate.yml
- Notas: Rodar scaffold, implementar apenas testes de contrato documental, executar verify e audit. Não declarar blockers resolvidos se a prova não estiver vinculada ao SHA/tree/policy. Depende de T-005.
