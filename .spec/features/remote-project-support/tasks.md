# Tasks: remote project support

> feature: remote-project-support

## T-3003 — Vincular projeto remoto ao node/peer com versão e capabilities bounded [concluida]

- Refs: US-3002, AC-3015, AC-3016, AC-3017, AC-3018, AC-3020, AC-3022
- Arquivos: `crates/remote-core/src/remote_project.rs`, `crates/remote-core/src/lib.rs`
- Notas: adiciona `RemoteProjectDescriptor` com IDs tipados e escopo bounded de capabilities,
  e `RemoteProjectRegistry` fail-closed com bind idempotente, conflito de versão e
  reconciliação estritamente mais nova; nenhum transport real é criado.

## T-3004 — Provar binding, rejeição, conflito e referências tipadas com fixture offline [concluida]

- Refs: US-3002, AC-3015, AC-3016, AC-3017, AC-3018, AC-3019, AC-3020, AC-3021, AC-3022
- Arquivos: `crates/remote-core/tests/remote_project_contract.rs`, `test/remote-project-support-onp.test.mjs`
- Notas: fixture determinística não usa rede, shell, provider real ou credencial real;
  referências são IDs tipados, nunca caminhos de filesystem crus.

## T-3005 — Registrar fronteira de produção e executar verify explícito [concluida]

- Refs: US-3002, AC-3015, AC-3016, AC-3017
- Arquivos: `.spec/features/remote-project-support/spec.md`, `.spec/features/remote-project-support/tasks.md`, `onpspec.config.json`, `docs/remote-project-support.md`
- Notas: `CONTRACT PASS` não é prova de sync/multi-master operacional; adapter de produção permanece posterior.

## Suposições

- ASM-2106 está registrada na especificação: sincronização e replicação multi-master são trabalho posterior.