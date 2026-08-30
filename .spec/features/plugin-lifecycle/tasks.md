# Tasks: plugin lifecycle

> feature: plugin-lifecycle

## T-1393 — Lifecycle state machine [concluida]

- Refs: US-1393, AC-1393
- Arquivos: crates/plugin-core/src/lifecycle.rs, crates/plugin-core/tests/lifecycle_contract.rs, docs/plugin-lifecycle.md

## T-1394 — Fail-closed failure handling [concluida]

- Refs: US-1394, AC-1394
- Arquivos: crates/plugin-core/src/lifecycle.rs, crates/plugin-core/tests/lifecycle_contract.rs, .github/workflows/onp-sdd-evidence.yml, test/aggregate-runner-native-boundary.js

## Suposições

- ASM-1393: adapters concretos de processo/worker serão implementados em etapa posterior e devem consumir este contrato sem ampliar autoridade.
