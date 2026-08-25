# Tasks: Explicit Skill rollback

> feature: skill-rollback

## T-845 — Implementar decisão de rollback idempotente [concluida]

- Refs: US-652, AC-841, AC-842, AC-843
- Arquivos: `crates/agent-runtime/src/skill_rollback.rs`, `crates/agent-runtime/src/lib.rs`, `crates/agent-runtime/tests/skill_rollback_contract.rs`, `.github/workflows/onp-sdd-evidence.yml`, `docs/skill-rollback.md`
- Notas: decisão bounded; operação transacional real, cache e bindings ficam fora desta fatia.
