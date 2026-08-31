# Tasks: planning evidence binding

> feature: planning-evidence-binding

## T-1420 — Resolver mapping e projeção de estado [concluída]

- Refs: US-1420, AC-1420, AC-1421, AC-1422
- Arquivos: crates/agent-core/src/planning_evidence_binding.rs,
  crates/agent-core/src/planning_reconciliation.rs,
  crates/agent-core/src/lib.rs,
  crates/agent-core/tests/planning_evidence_binding_contract.rs
- Escopo: mapear `ReviewerFinding` e `EvidenceRecord` para `Claim`, preservar
  estados factuais e bloquear mitigação sem prova verificada.

## T-1423 — Lifecycle, bounds e observabilidade [concluída]

- Refs: US-1423, AC-1423, AC-1424
- Arquivos: crates/agent-core/src/planning_evidence_binding.rs,
  crates/agent-core/tests/planning_evidence_binding_contract.rs,
  docs/planning-evidence-binding.md,
  .github/workflows/onp-sdd-evidence.yml,
  test/aggregate-runner-native-boundary.js
- Escopo: cancelamento, replay determinístico, rejeição de órfãos/schema
  desconhecido, métricas bounded e documentação.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.
