# Tasks: Non-activating Skill evaluation

> feature: skill-evaluation

## T-819 — Implementar evaluator bounded e não ativante [concluida]

- Refs: US-648, AC-810, AC-811, AC-812, AC-813, AC-814, AC-815, AC-816
- Arquivos: `crates/agent-runtime/src/skill_evaluation.rs`, `crates/agent-runtime/src/lib.rs`, `crates/agent-runtime/tests/skill_evaluation_contract.rs`, `.github/workflows/onp-sdd-evidence.yml`, `docs/skill-evaluation.md`
- Notas: compara baseline e candidata em memória; reconfirma validação,
  capability, escopo, identidade e digests de fixtures; estados não passantes
  não têm caminho de ativação e o relatório contém somente metadados bounded.
