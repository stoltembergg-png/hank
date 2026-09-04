# Tasks: crash recovery contract

> feature: crash-recovery-contract

## T-1500 — Startup recovery coordinator fail-closed [pendente]

- Refs: US-1500, AC-1501, AC-1502, AC-1503, AC-1504, AC-1505
- Arquivos: crates/recovery-core/src/lib.rs, crates/recovery-core/src/coordinator.rs, crates/recovery-core/src/marker.rs, crates/recovery-core/src/storage.rs, crates/recovery-core/tests/crash_recovery_contract.rs, crates/recovery-core/Cargo.toml, docs/crash-recovery-contract.md

## Suposições

- ASM-1500: o `RecoveryStorage` trait é injetado; este card entrega o contrato
  e um `InMemoryStorage` para os testes. Adapters concretos (SQLite, sled,
  fsync) pertencem a PRs posteriores.
- ASM-1501: nenhum efeito irreversível é executado por este card. O
  coordinator confia no storage; ele próprio não reescreve o marker.
- ASM-1502: testes de property/fuzz para entradas adversariais pertencem a
  PRs subsequentes; este card entrega unit tests determinísticos.
