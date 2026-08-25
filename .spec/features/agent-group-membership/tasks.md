# Tasks: AgentGroup membership

> feature: agent-group-membership

## T-870 — Implementar membership scoped e snapshot rollbackable [concluida]

- Refs: US-865, AC-866, AC-867, AC-868
- Arquivos: `crates/agent-core/src/group_entity.rs`, `crates/agent-core/tests/group_membership_contract.rs`, `docs/agent-group-membership.md`
- Notas: add/remove são bounded e não executam agentes; persistência transacional fica posterior.
