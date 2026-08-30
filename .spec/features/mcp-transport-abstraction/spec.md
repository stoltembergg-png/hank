# Spec: MCP transport abstraction

> feature: mcp-transport-abstraction
> status: auditada

### US-1377 — Versioned bounded transport

Como cliente MCP, quero negociar protocolo e capabilities com framing limitado.

#### AC-1377 — Handshake and frames

- **Dado** versão/capabilities suportadas e frame dentro do limite.
- **Quando** o envelope é validado.
- **Então** handshake e frame são aceitos preservando correlation ID.
- **Dado** versão desconhecida, frame oversized ou correlation inválida.
- **Quando** validado.
- **Então** retorna erro tipado fail-closed.

### US-1378 — Deterministic lifecycle

Como transporte MCP, quero cancelar, fechar e classificar reconnect sem loops ilimitados.

#### AC-1378 — Lifecycle and backpressure

- **Dado** sessão ativa.
- **Quando** cancel/close é solicitado.
- **Então** a transição é determinística e idempotente.
- **Dado** fila no limite ou reconnect não permitido.
- **Quando** uma operação é solicitada.
- **Então** retorna backpressure/reconnect error sem aceitar trabalho adicional.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Contrato MCP transport-neutral, versionado, bounded e fail-closed, sem transporte concreto.
