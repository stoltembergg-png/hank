# Tasks: Skill lifecycle curator

> feature: skill-lifecycle-curator

## T-850 — Implementar curator puro e fail-closed [concluida]

- Refs: US-653, AC-846, AC-847, AC-848
- Arquivos: `crates/agent-runtime/src/skill_lifecycle_curator.rs`, `crates/agent-runtime/src/lib.rs`, `crates/agent-runtime/tests/skill_lifecycle_curator_contract.rs`, `.github/workflows/onp-sdd-evidence.yml`, `docs/skill-lifecycle-curator.md`
- Notas: primeira fatia centraliza decisões; persistência/eventos/rollback efetivo não são alterados.
