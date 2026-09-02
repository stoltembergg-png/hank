# Tasks: remote protocol

> feature: remote-protocol

## T-1450 — Protocol handshake and negotiation [em-andamento]

- Refs: US-1448, AC-1453
- Arquivos: crates/agent-protocol/src/remote_protocol.rs, crates/agent-protocol/src/lib.rs,
  crates/agent-protocol/tests/remote_protocol_contract.rs, docs/remote-protocol.md

## T-1451 — Command catalog, correlation and error model [em-andamento]

- Refs: US-1448, AC-1454, AC-1455, AC-1456
- Arquivos: crates/agent-protocol/src/remote_protocol.rs,
  crates/agent-protocol/tests/remote_protocol_contract.rs,
  .github/workflows/onp-sdd-evidence.yml,
  test/aggregate-runner-native-boundary.js

## Suposições

- ASM-1453: auth, WebSocket, remote tool dispatch e credential isolation pertencem
  aos cards PR-246+.