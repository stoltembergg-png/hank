# Tasks: planning reconciliation

> feature: planning-reconciliation

## T-1403 — Reconciliation contract [concluida]

- Refs: US-1403, AC-1403, AC-1404, AC-1405
- Arquivos: crates/agent-core/src/planning_reconciliation.rs, crates/agent-core/src/lib.rs, crates/agent-core/tests/planning_reconciliation_contract.rs, docs/planning-reconciliation.md
- Escopo: matriz de disposições, dedupe com provenance e escalada HUMAN_REQUIRED.

## T-1406 — Safety and lifecycle boundaries [concluida]

- Refs: US-1403, AC-1406, AC-1407, AC-1408
- Arquivos: crates/agent-core/src/planning_reconciliation.rs, crates/agent-core/tests/planning_reconciliation_contract.rs, .github/workflows/onp-sdd-evidence.yml, test/aggregate-runner-native-boundary.js
- Escopo: bounds, identidade, schema, não autoridade, cancelamento, idempotência e reopen.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.
