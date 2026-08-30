# Spec: MCP HTTP client

> feature: mcp-http-client
> status: auditada

### US-1381 — Policy-bound HTTP MCP request

Como cliente MCP, quero enviar requests HTTP somente a endpoints e schemes autorizados.

#### AC-1381 — Endpoint, TLS and limits

- **Dado** endpoint HTTPS allowlisted sem credenciais na URL e limites válidos.
- **Quando** a configuração é validada.
- **Então** é aceita com streaming bounded.
- **Dado** host não allowlisted, HTTP sem policy explícita, URL com credentials ou body oversized.
- **Quando** validado.
- **Então** retorna erro tipado fail-closed.

### US-1382 — Safe retry and cancellation

Como transporte MCP HTTP, quero retry somente quando seguro e cancelar requests em voo.

#### AC-1382 — Idempotency and cancellation

- **Dado** método GET/PUT/DELETE idempotente.
- **Quando** retry é classificado.
- **Então** é permitido dentro do limite.
- **Dado** POST sem idempotency key ou retry limit excedido.
- **Quando** classificado.
- **Então** é negado.
- **Dado** request cancelado.
- **Quando** avaliado.
- **Então** retorna cancelamento sem aceitar novo trabalho.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Contrato MCP HTTP bounded sobre a política HTTP existente; sem armazenamento de credentials ou dependência de internet pública.
