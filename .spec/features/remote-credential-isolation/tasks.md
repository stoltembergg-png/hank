# Tasks: remote credential isolation

> feature: remote-credential-isolation

## T-1461 — Broker de credencial remoto scoped e redigido [pendente]

- Refs: US-1451, AC-1466, AC-1467, AC-1468, AC-1469, AC-1470
- Arquivos: crates/remote-core/src/credential_broker.rs, crates/remote-core/src/lib.rs,
  crates/remote-core/tests/credential_broker_contract.rs,
  .github/workflows/onp-sdd-evidence.yml, test/aggregate-runner-native-boundary.js,
  docs/remote-credential-isolation.md

## Suposições

- ASM-1461: adapters concretos de OS keychain/Stronghold, transporte de
  referência e migração de secrets existentes (PR-256) pertencem a cards
  posteriores, mantendo este core sem material de segredo.
