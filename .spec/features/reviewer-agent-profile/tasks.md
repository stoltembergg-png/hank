# Tasks: Reviewer agent profile

> feature: reviewer-agent-profile

## T-1325 — Implementar reviewer read-only com evidence bound [concluída]

- Refs: US-1325, US-1326, US-1327, AC-1325, AC-1326, AC-1327
- Escopo: profile, request/permit read-only, SHA/tree identity, findings, evidence e handoff advisory.
- Arquivos: crates/agent-core/src/reviewer_profile.rs, crates/agent-core/tests/reviewer_agent_profile_contract.rs, crates/agent-core/src/lib.rs, docs/reviewer-agent-profile.md, .github/workflows/onp-sdd-evidence.yml, test/aggregate-runner-native-boundary.js
- Não-escopo: execução de ferramentas, provider, GitHub, Git, filesystem, rede, secrets, alteração de gate, CODEOWNERS, Ruleset, aprovação, merge, QA ou security profile.
- Testes: scope/tool deny, exact SHA/tree, stale/missing/skipped/no-run/malformed evidence, digest bounds, prompt-injection data-only e não autoridade.
- Gates: fmt, check, test, clippy, build, feature runner, ONP verify/audit e CI required checks.
- Rollback: revert aditivo sem migration ou estado externo.
