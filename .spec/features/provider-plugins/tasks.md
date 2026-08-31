# Tasks: provider plugins

> feature: provider-plugins

## T-1397 — Normalized provider adapter [concluida]

- Refs: US-1397, AC-1397
- Arquivos: crates/provider-core/src/plugin_adapter.rs, crates/provider-core/tests/plugin_adapter_contract.rs, docs/provider-plugins.md

## T-1398 — Fail-closed plugin boundary [concluida]

- Refs: US-1397, AC-1398
- Arquivos: crates/provider-core/src/plugin_adapter.rs, crates/provider-core/tests/plugin_adapter_contract.rs, .github/workflows/onp-sdd-evidence.yml, test/aggregate-runner-native-boundary.js

## Suposições

- ASM-1397: aprovação, permissions e lifecycle são decisões externas; este adapter não as concede.
