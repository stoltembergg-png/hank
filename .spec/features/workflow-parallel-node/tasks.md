# Tasks: ParallelNode planning contract

> feature: workflow-parallel-node

## T-1000 — Implementar plano bounded de ParallelNode [concluida]

- Refs: US-994, AC-995, AC-996, AC-997
- Arquivos: `crates/workflow-core/src/parallel.rs`, `crates/workflow-core/tests/parallel_contract.rs`
- Escopo: validação de fan-out/cap, join all/any/quorum, resultado ordenado e cancellation state.
