# Spec: remote protocol

> feature: remote-protocol
> status: em-implementacao

### US-1448 — Remote protocol negotiation and command catalog

Como runtime remoto, quero negociar versão de protocolo, capabilities e comandos com um peer,
para que toda comunicação tenha identity, correlação e ordenação bounded antes de qualquer
execução remota ou autenticação.

#### AC-1453 — Handshake negotiates protocol, API and capabilities

- **Dado** handshake com protocol version, API revision e capabilities bounded
- **Quando** é negociado contra a versão local
- **Então** major matching, minor compatível e capabilities conhecidas produzem NegotiatedProtocol;
  protocol major desconhecido, API incompatible ou capability desconhecida são rejeitados.

#### AC-1454 — Typed command catalog rejects unknown commands

- **Dado** catálogo tipado com comandos known (ping, get_state, subscribe, cancel)
- **Quando** um comando é consultado
- **Então** known command retorna CommandSpec com idempotency flag;
  unknown command produz UnknownCommand.

#### AC-1455 — Correlation tracking rejects duplicate, stale and out-of-order

- **Dado** RequestTracker bounded com estados pending/terminal
- **Quando** há begin duplicado, complete após cancel, cancel após complete,
  event sequence ≤ last sequence, ou request_id unknown
- **Então** DuplicateCorrelation, StaleCorrelation, OutOfOrder e
  UnknownCorrelation são fail-closed.

#### AC-1456 — Identity mismatch and oversized payload are fail-closed

- **Dado** handshake com peer/node identity divergente da esperada,
  ou mensagem serializada exceedendo MAX_PAYLOAD
- **Quando** identity mismatch é verificado ou payload bound é verificado
- **Então** IdentityMismatch ou PayloadTooLarge rejeitam a operação.

## Segurança

- Capabilities são negociadas, não concedidas: capability desconhecida do peer é rejeitada,
  não silenciada.
- Handshake não carrega credenciais; identidade é bounded e validada.
- Correlation tracking não expõe payload.

## Suposições

- ASM-1453: auth, WebSocket, remote tool dispatch e credential isolation pertencem
  aos cards PR-246+.

## Perguntas em aberto

Nenhuma.