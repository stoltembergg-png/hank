# Tasks: Provenance-bound Skill candidate generation

> feature: skill-candidate

## T-824 — Implementar geração de candidate sem ativação [concluida]

- Refs: US-649, AC-817, AC-818, AC-819, AC-820, AC-821, AC-822, AC-823
- Arquivos: `crates/agent-runtime/src/skill_candidate.rs`,
  `crates/agent-runtime/src/lib.rs`,
  `crates/agent-runtime/tests/skill_candidate_contract.rs`,
  `.github/workflows/onp-sdd-evidence.yml`, `docs/skill-candidate.md`
- Notas: normaliza observações, valida contexto bounded, usa o parser sem
  persistência, quarentena conteúdo hostil/escalado, produz handoff hash-only
  para o evaluator e mantém descarte idempotente em memória.
