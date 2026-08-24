# Tasks: Governed Skill creation

> feature: skill-creation

## T-812 — Implementar criação e descarte de Draft governados [concluida]

- Refs: US-647, AC-803, AC-804, AC-805, AC-806, AC-807, AC-808, AC-809
- Arquivos: `crates/agent-runtime/src/skill_creation.rs`, `crates/agent-runtime/src/lib.rs`, `crates/agent-runtime/tests/skill_creation_contract.rs`, `crates/agent-runtime/tests/skill_creation_tool_contract.rs`, `.github/workflows/onp-sdd-evidence.yml`, `docs/skill-creation.md`
- Notas: criação é project-scoped, Draft-only, idempotente e redigida; parser,
  fixture runner e validação falham fechados antes da persistência; nenhuma
  operação executa conteúdo ou altera uma versão ativa.
