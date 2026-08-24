# Tasks: Governed Skill validation

> feature: skill-validation

## T-805 — Implementar validação e gate de lifecycle [concluida]

- Refs: US-646, AC-796, AC-797, AC-798, AC-799, AC-800, AC-801, AC-802
- Arquivos: `crates/agent-runtime/src/skill_validation.rs`, `crates/agent-runtime/src/skill_repo.rs`, `crates/agent-runtime/tests/skill_validation_contract.rs`, `crates/agent-runtime/tests/skill_repository_contract.rs`, `crates/agent-runtime/tests/skill_loader_contract.rs`, `crates/agent-runtime/tests/skill_editor_contract.rs`, `crates/agent-runtime/tests/skill_versioning_contract.rs`, `.github/workflows/onp-sdd-evidence.yml`, `docs/skill-validation.md`
- Notas: validação somente em memória, sem resolução/execução/mutação de conteúdo; promoção e rollback exigem relatório determinístico redigido.
