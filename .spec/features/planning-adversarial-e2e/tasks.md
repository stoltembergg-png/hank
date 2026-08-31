# Tasks: adversarial planning E2E

> feature: planning-adversarial-e2e

## T-1430 — Pipeline plan-to-final [concluída]

- Refs: US-1430, AC-1430
- Arquivos: crates/agent-core/tests/planning_adversarial_e2e.rs
- Escopo: fixture de plano e reviewers virtuais, reconciliação e binding de
  evidência verificada.

## T-1431 — Dedupe e corpus adversarial [concluída]

- Refs: US-1431, US-1432, AC-1431, AC-1432
- Arquivos: crates/agent-core/tests/planning_adversarial_e2e.rs
- Escopo: duplicata com discordância, reviewer hostil, conflito crítico,
  self-review, round overflow e orçamento bounded.

## T-1434 — Replay, cancelamento e documentação [concluída]

- Refs: US-1433, US-1434, AC-1433, AC-1434
- Arquivos: crates/agent-core/tests/planning_adversarial_e2e.rs,
  docs/planning-adversarial-e2e.md,
  .github/workflows/onp-sdd-evidence.yml,
  test/aggregate-runner-native-boundary.js
- Escopo: identidade da evidência, cancelamento, idempotência e gates ONP.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.
