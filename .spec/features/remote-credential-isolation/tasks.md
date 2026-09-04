# Tasks: remote credential isolation

> feature: remote-credential-isolation

## T-1461 — Broker de credencial remoto scoped e redigido [concluída]

- Refs: US-1451, AC-1466, AC-1467, AC-1468, AC-1469, AC-1470
- Arquivos: crates/remote-core/src/credential_broker.rs, crates/remote-core/src/lib.rs,
  crates/remote-core/tests/credential_broker_contract.rs,
  .github/workflows/onp-sdd-evidence.yml, test/aggregate-runner-native-boundary.js,
  docs/remote-credential-isolation.md

## T-1462 — Seed de handle via CSPRNG no adapter [concluída]

- Refs: US-1451, AC-1466
- Arquivos: crates/remote-core/src/credential_broker.rs,
  crates/remote-core/tests/credential_broker_contract.rs,
  crates/remote-adapter/src/lib.rs, crates/remote-adapter/Cargo.toml,
  Cargo.toml, Cargo.lock, crates/test-support/src/arch_fixtures_test.rs,
  docs/remote-credential-isolation.md
- O core recebe somente `BrokerEntropy`; o adapter usa `getrandom` sem fallback
  temporal/contador e retorna erro tipado quando o CSPRNG não está disponível.

## Suposições

- ASM-1461: adapters concretos de OS keychain/Stronghold, transporte de
  referência e migração de secrets existentes (PR-256) pertencem a cards
  posteriores, mantendo este core sem material de segredo.
