# Spec: runtime transport

> feature: runtime-transport
> status: auditada

### US-1401 — Runtime-neutral transport contract

Como runtime, quero transportar envelopes versionados entre peers sem acoplar o protocolo a socket, daemon ou autenticação.

#### AC-1401 — Bounded framing and identity

- **Dado** envelope com versão, conexão, sessão e correlação válidas dentro dos limites
- **Quando** é aceito
- **Então** permanece identificado e bounded; versão desconhecida, identidade ausente ou frame oversized são rejeitados.

#### AC-1402 — Lifecycle and backpressure

- **Dado** sessão ativa com fila limitada
- **Quando** há cancelamento, fechamento, fila cheia ou reconnect
- **Então** cancelamento/fechamento são idempotentes, backpressure é fail-closed e reconnect não autorizado é rejeitado.

## Segurança

- Este contrato não autentica peers, não abre sockets, não cria daemon e não executa tools.
- Identidades e capabilities são dados declarativos; dispatch pertence a etapa posterior.
- Buffers, frames e reconnects são bounded.

## Suposições

- ASM-1401: transportes concretos e autenticação serão adapters posteriores que consumirão este contrato.

## Perguntas em aberto

Nenhuma.
