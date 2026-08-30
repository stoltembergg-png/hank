# Tasks: plugin discovery

> feature: plugin-discovery

## T-1391 — Bounded source staging [concluida]

- Refs: US-1391, AC-1391
- Arquivos: crates/plugin-core/src/discovery.rs, crates/plugin-core/tests/discovery_contract.rs, docs/plugin-discovery.md

## T-1392 — Fail-closed source policy [concluida]

- Refs: US-1392, AC-1392
- Arquivos: crates/plugin-core/src/discovery.rs, crates/plugin-core/tests/discovery_contract.rs, .github/workflows/onp-sdd-evidence.yml, test/aggregate-runner-native-boundary.js

## Suposições

- ASM-1391: a entrada do adapter já está limitada a metadados e não representa bytes executáveis.
