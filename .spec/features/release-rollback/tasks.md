# Tasks: release rollback

> feature: release-rollback

## T-3006 — Modelar proof verified, slot known-good e decisão de rollback [concluida]

- Refs: US-3003, AC-3806, AC-3810, AC-3811, AC-3813
- Arquivos: `crates/recovery-core/src/rollback.rs`, `crates/recovery-core/src/lib.rs`
- Notas: adiciona `ReleaseProof` (identidade/digest bounded, redigido de Debug/Display),
  `KnownGoodSlot` e avanço estritamente mais novo; nenhum updater/signer real é criado.

## T-3007 — Provar seleção, revogação, convergência e redação com fixture offline [concluida]

- Refs: US-3003, AC-3807, AC-3808, AC-3809, AC-3812
- Arquivos: `crates/recovery-core/tests/release_rollback_contract.rs`, `test/release-rollback-onp.test.mjs`
- Notas: fixture determinística não usa rede, shell, signer, filesystem ou credencial real;
  rollback convergente e bounded, versão revogada nunca restaurada.

## T-3008 — Registrar fronteira de produção e executar verify explícito [concluida]

- Refs: US-3003, AC-3807, AC-3808, AC-3809
- Arquivos: `.spec/features/release-rollback/spec.md`, `.spec/features/release-rollback/tasks.md`, `onpspec.config.json`, `docs/release-rollback.md`
- Notas: `CONTRACT PASS` não é prova de rollback operacional; operação de signer e publicação automatizada permanecem posteriores.

## Suposições

- ASM-2107 está registrada na especificação: operação de chave de signer e publicação automatizada são trabalho posterior.