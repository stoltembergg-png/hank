# Tasks: Group budget accounting

> feature: group-budget

## T-914 — Adicionar reservation/reconciliation group-scoped [concluida]

- Refs: US-908, AC-910, AC-911, AC-912
- Arquivos: `crates/agent-core/src/group_budget.rs`, `crates/agent-core/src/lib.rs`, `crates/agent-core/tests/group_budget_contract.rs`, `docs/group-budget.md`
- Notas: reutiliza `BudgetAccount`; não altera budget global nem inicia execução.
