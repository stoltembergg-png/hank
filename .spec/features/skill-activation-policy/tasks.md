# Tasks: Governed Skill activation policy

> feature: skill-activation-policy

## T-840 — Implementar decisão de ativação fail-closed [concluida]

- Refs: US-651, AC-836, AC-837, AC-838
- Arquivos: `crates/agent-runtime/src/skill_activation_policy.rs`, `crates/agent-runtime/src/lib.rs`, `crates/agent-runtime/tests/skill_activation_policy_contract.rs`, `.github/workflows/onp-sdd-evidence.yml`, `docs/skill-activation-policy.md`
- Notas: decisão pura; não persiste ponteiro, não ativa e exige evidência bounded.
