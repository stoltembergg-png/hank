# Tasks: plugin permissions

> feature: plugin-permissions

## T-1395 — Exact plugin capability policy [concluida]

- Refs: US-1395, AC-1395
- Arquivos: crates/security-core/src/plugin_permission.rs, crates/security-core/tests/plugin_permission_contract.rs, docs/plugin-permissions.md

## T-1396 — Revoke and upgrade re-consent [concluida]

- Refs: US-1395, AC-1396
- Arquivos: crates/security-core/src/plugin_permission.rs, crates/security-core/tests/plugin_permission_contract.rs, .github/workflows/onp-sdd-evidence.yml, test/aggregate-runner-native-boundary.js

## Suposições

- ASM-1395: a decisão não executa efeitos e não acessa secrets; adapters externos permanecem fora do domínio.
