# Tasks: Agent invocation protocol

> feature: agent-invocation-protocol

## T-886 — Implementar contrato provider-agnostic bounded [concluida]

- Refs: US-881, AC-882, AC-883, AC-884
- Arquivos: `crates/agent-protocol/src/invocation.rs`, `crates/agent-protocol/src/lib.rs`, `crates/agent-protocol/tests/invocation_contract.rs`, `docs/agent-invocation-protocol.md`
- Notas: request é validável e serializável; não cria invocation real nem chama transport.
