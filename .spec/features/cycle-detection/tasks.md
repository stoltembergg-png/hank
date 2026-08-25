# Tasks: Cycle detection

> feature: cycle-detection

## T-900 — Detectar self-loop e ancestry cycle bounded [concluida]

- Refs: US-895, AC-897, AC-898, AC-899
- Arquivos: `crates/agent-core/src/cycle_detection.rs`, `crates/agent-core/src/invocation_graph.rs`, `crates/agent-core/tests/cycle_detection_contract.rs`, `docs/cycle-detection.md`
- Notas: detector é read-only; não registra edge rejeitada nem chama execução.
