# Tasks: websocket event stream

> feature: websocket-event-stream

## T-1460 — Contrato de event stream transport-neutral [concluida]

- Refs: US-1450, AC-1461, AC-1462, AC-1463, AC-1464, AC-1465
- Arquivos: crates/remote-core/src/event_stream.rs, crates/remote-core/src/lib.rs,
  crates/remote-core/tests/event_stream_contract.rs,
  .github/workflows/onp-sdd-evidence.yml, test/aggregate-runner-native-boundary.js,
  docs/websocket-event-stream.md

## Suposições

- ASM-1460: adapters concretos de WebSocket, TLS, bind e dispatch de eventos
  pertencem a cards posteriores, mantendo este core sem dependência de rede.