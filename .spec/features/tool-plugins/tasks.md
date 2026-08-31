# Tasks: tool plugins

> feature: tool-plugins

## T-1399 — Permissioned tool adapter [concluida]

- Refs: US-1399, AC-1399
- Arquivos: crates/tool-core/src/plugin_tool.rs, crates/tool-core/tests/plugin_tool_contract.rs, docs/tool-plugins.md

## T-1400 — Fail-closed tool boundary [concluida]

- Refs: US-1399, AC-1400
- Arquivos: crates/tool-core/src/plugin_tool.rs, crates/tool-core/tests/plugin_tool_contract.rs, .github/workflows/onp-sdd-evidence.yml, test/aggregate-runner-native-boundary.js

## Suposições

- ASM-1399: o contexto de execução contém a decisão de permission e os budgets; o wrapper não faz grants implícitos.
