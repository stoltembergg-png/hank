# Tasks: AgentGroup repository

> feature: agent-group-repository

## T-865 — Persistir AgentGroup com scope e revisão otimista [concluida]

- Refs: US-859, AC-861, AC-862, AC-863
- Arquivos: `migrations/0012_agent_groups.sql`, `crates/agent-runtime/src/agent_group_repo.rs`, `crates/agent-runtime/src/lib.rs`, `crates/agent-runtime/tests/agent_group_repository_contract.rs`, `docs/agent-group-repository.md`
- Notas: queries project-scoped, JSON policy-preserving, duplicate/stale fail-closed e archive idempotente.
