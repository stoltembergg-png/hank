# Tasks: Coding agent profile

> feature: coding-agent-profile

## T-1322 — Implementar profile coding bounded e handoff verificável [concluída]

- Refs: US-1322, US-1323, AC-1322, AC-1323, AC-1324
- Arquivos: crates/agent-core/src/coding_profile.rs, crates/agent-core/src/lib.rs, crates/agent-core/tests/coding_agent_profile_contract.rs, docs/coding-agent-profile.md, .github/workflows/onp-sdd-evidence.yml, test/aggregate-runner-native-boundary.js, .spec/features/coding-agent-profile/spec.md, .spec/features/coding-agent-profile/tasks.md
- Escopo: novo contrato puro em `agent-core`, testes de decisão/handoff,
  documentação e inclusão da verificação ONP no workflow existente.
- Não-escopo: executor, provider, GitHub, Git, filesystem, rede, commit, PR,
  merge, release ou mudança de capabilities reais.
- Gates: `cargo fmt`, `cargo test --workspace --locked`, Clippy com warnings,
  feature runner, aggregate boundary, ONP verify e audit global classificado.
- Rollback: remover somente os arquivos do profile e sua verificação; nenhum
  schema ou efeito externo é alterado.
