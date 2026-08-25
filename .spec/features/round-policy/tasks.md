# Tasks: Round policy

> feature: round-policy

## T-924 — Controlar rounds, turns, no-progress e retry [concluida]

- Refs: US-918, AC-920, AC-921, AC-922
- Arquivos: `crates/agent-core/src/round_policy.rs`, `crates/agent-core/src/lib.rs`, `crates/agent-core/tests/round_policy_contract.rs`, `docs/round-policy.md`
- Notas: state machine pura; não cria loops nem executa modelos.
