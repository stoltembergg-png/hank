# Tasks: claim and evidence contract

> feature: claim-evidence

## T-1410 — Claim/Evidence domain contract [concluída]

- Refs: US-1410, AC-1410, AC-1411, AC-1412
- Arquivos: crates/agent-core/src/claim_evidence.rs, crates/agent-core/src/lib.rs, crates/agent-core/tests/claim_evidence_contract.rs
- Escopo: claims por digest, EvidenceScope, EvidenceRecord, estados factuais,
  transições fail-closed, replay idempotente e requisitos de evidência.

## T-1413 — Versioning, bounds and architecture boundary [concluída]

- Refs: US-1413, AC-1413, AC-1414, AC-1415, AC-1416
- Arquivos: crates/agent-core/src/claim_evidence.rs, crates/agent-core/tests/claim_evidence_contract.rs, docs/claim-evidence.md, .github/workflows/onp-sdd-evidence.yml, test/aggregate-runner-native-boundary.js
- Escopo: serde versionado com campos desconhecidos rejeitados, limites,
  redaction de metadata sensível, identidade de projeto/run/trace/SHA/tree e
  ausência de autoridade operacional.

## Fora de escopo

- resolver externo, persistência, UI, scheduler e binding de findings;
- qualquer alteração de estado factual por texto de modelo ou memória.
