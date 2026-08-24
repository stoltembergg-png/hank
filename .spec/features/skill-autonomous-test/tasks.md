# Tasks: Autonomous bounded Skill testing

> feature: skill-autonomous-test

## T-835 — Implementar execução autônoma bounded não mutante [concluida]

- Refs: US-650, AC-831, AC-832, AC-833
- Arquivos: `crates/agent-runtime/src/skill_autonomous_test.rs`, `crates/agent-runtime/src/lib.rs`, `crates/agent-runtime/tests/skill_autonomous_test_contract.rs`, `.github/workflows/onp-sdd-evidence.yml`, `docs/skill-autonomous-test.md`
- Notas: primeira fatia usa somente fixture declarativa e sandbox lógico project-scoped; não executa host effects nem ativa Skill.
