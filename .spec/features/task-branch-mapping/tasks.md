# Tasks: Task-to-branch mapping

> feature: task-branch-mapping

## T-1317 — Implementar contrato, lifecycle e persistência do task mapping [concluida]
- Refs: US-1317, AC-1317, AC-1318, US-1319, AC-1319, AC-1320, US-1321, AC-1321
- Arquivos: crates/agent-core/src/task_mapping.rs, crates/agent-core/src/lib.rs, crates/agent-core/tests/task_mapping_contract.rs, crates/agent-runtime/src/task_mapping_repo.rs, crates/agent-runtime/src/lib.rs, crates/agent-runtime/tests/task_mapping_repository_contract.rs, migrations/0021_task_workspace_mappings.sql, docs/task-branch-mapping.md, docs/branch-policy.md, docs/migrations.md, .github/workflows/onp-sdd-evidence.yml, test/aggregate-runner-native-boundary.js
- Modelo: sonnet
- Esforço: alto
- Notas: um único slice vertical, sem UI/Git/capability; domínio puro primeiro, depois repository SQLite com compare-and-set e roundtrip.
