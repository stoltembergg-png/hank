# Tasks: Parallel invocation batch

> feature: parallel-invocation

## T-909 — Preparar batch paralelo bounded [concluida]

- Refs: US-903, AC-905, AC-906, AC-907
- Arquivos: `crates/agent-core/src/parallel_batch.rs`, `crates/agent-core/src/lib.rs`, `crates/agent-core/tests/parallel_batch_contract.rs`, `docs/parallel-invocation.md`
- Notas: planner read-only quanto a workers; cycle/depth gates são entradas obrigatórias.
