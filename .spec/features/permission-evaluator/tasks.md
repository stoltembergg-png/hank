# Tasks: Permission evaluator

> feature: permission-evaluator

## T-628 — Implementar evaluator bounded e fail-closed [concluida]

- Refs: US-605, AC-632, AC-633, AC-634
- Arquivos: crates/tool-core/src/permission.rs, crates/tool-core/src/lib.rs, crates/tool-core/tests/permission_contract.rs
- Notas: deny default, policy matrix, capability/budget/identity validation, ask_once scoped cache, ask_every_time, concurrency e clear project.

## T-629 — Documentar contrato de autorização [concluida]

- Refs: US-605, AC-632, AC-633
- Arquivos: docs/permission-evaluator.md
- Notas: precedência, efeitos confirmáveis, isolamento e limites.
