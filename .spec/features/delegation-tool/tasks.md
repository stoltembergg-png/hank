# Tasks: Delegation tool

> feature: delegation-tool

## T-891 — Criar builder Pending e ledger dedupe/cancel [concluida]

- Refs: US-886, AC-887, AC-888, AC-889
- Arquivos: `crates/tool-core/src/delegation.rs`, `crates/tool-core/src/lib.rs`, `crates/tool-core/tests/delegation_tool_contract.rs`, `crates/agent-protocol/src/invocation.rs`, `docs/delegation-tool.md`
- Notas: tool não executa worker/provider; graph e scheduler permanecem posteriores.
