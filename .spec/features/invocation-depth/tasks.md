# Tasks: Maximum invocation depth

> feature: invocation-depth

## T-904 — Validar depth contra ancestry e máximo bounded [concluida]

- Refs: US-899, AC-901, AC-902, AC-903
- Arquivos: `crates/agent-core/src/depth_limit.rs`, `crates/agent-core/src/invocation_graph.rs`, `crates/agent-core/tests/depth_limit_contract.rs`, `docs/invocation-depth.md`
- Notas: preflight read-only; não registra nem executa requests.
