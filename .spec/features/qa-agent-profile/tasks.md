# Tasks: QA agent profile

> feature: qa-agent-profile

## T-1328 — Implementar QA profile e evidence bound [concluída]

- Refs: US-1328, US-1329, US-1330, AC-1328, AC-1329, AC-1330
- Escopo: contrato puro em `agent-core` para plano QA tipado, allowlist de
  comandos, limites de timeout/tentativas/output, parser de resultados,
  identidade SHA/tree, digests de output/artefato e failure handoff advisory.
- Arquivos: crates/agent-core/src/qa_profile.rs, crates/agent-core/src/lib.rs, crates/agent-core/tests/qa_agent_profile_contract.rs, docs/qa-agent-profile.md, .github/workflows/onp-sdd-evidence.yml, test/aggregate-runner-native-boundary.js, .spec/features/qa-agent-profile/spec.md, .spec/features/qa-agent-profile/tasks.md
- Não-escopo: executor de processo, shell, Git, filesystem, provider, CI
  remoto, secrets, alteração de expectations, desativação de testes ou decisão
  de release.
- Testes: allowlist/instruction injection, scope/state, timeout/resource bounds,
  wrong SHA/tree, missing/skipped/no-run/malformed/stale results, artifact
  digest e failure handoff.
- Gates: fmt, check, test, clippy, build, feature runner, aggregate boundary,
  ONP verify/audit e CI required checks.
- Rollback: remover somente o módulo, contrato, docs e sua verificação ONP;
  nenhum schema ou efeito externo é alterado.
