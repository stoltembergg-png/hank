# Spec: MCP stdio client

> feature: mcp-stdio-client
> status: auditada

### US-1379 — Bounded local stdio adapter

Como sistema, quero conversar com servidor MCP local declarado via stdio.

#### AC-1379 — Command and lifecycle safety

- **Dado** executable absoluto na allowlist e argv explícito.
- **Quando** o cliente é configurado.
- **Então** a configuração é aceita sem shell interpolation e com buffers bounded.
- **Dado** path não autorizado, argumento inválido ou output oversized.
- **Quando** configurado/recebido.
- **Então** retorna erro tipado fail-closed.

### US-1380 — Timeout, cancellation and restart

Como cliente MCP, quero interromper servidores que travam ou falham.

#### AC-1380 — Deterministic process policy

- **Dado** sessão ativa.
- **Quando** timeout/cancel/crash ocorre.
- **Então** a sessão termina com motivo observável e cleanup bounded.
- **Dado** restart solicitado.
- **Quando** o limite de tentativas foi atingido.
- **Então** o restart é negado.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Adapter stdio bounded e declarado; sem shell arbitrário, discovery ou confiança automática.
