# Spec: Worker protocol

> feature: worker-protocol
> status: em-implementacao

## Contexto

PR-112 define o contrato versionado e mínimo entre o Agent Runtime e um
worker Python opcional. O contrato é neutro: mensagens, identidade, ciclo de
vida e validação fail-closed vivem no `agent-protocol`, sem processo
Python, transporte JSON-RPC ou SDK. O core continua compilando e operando
sem Python (D-006, AI-016).

## Histórias

### US-619 — Conversar com worker opcional por contrato versionado

Como Agent Runtime, quero um contrato de mensagens versionado, bounded e
correlacionável com o worker Python opcional, para que a extensão Python
exista sem acoplar o core a processo, SDK ou transporte.

#### AC-677 — Handshake e ciclo de vida explícitos com serialização determinística

- **Dado** uma conversa runtime ↔ worker iniciada por handshake
- **Quando** as mensagens são trocadas na ordem do ciclo de vida (handshake,
  accept, request, response, health, shutdown, ack)
- **Então** a sessão aceita exatamente essa sequência e a serialização de
  cada mensagem é determinística com a versão vigente do schema

#### AC-678 — Ordenação fail-closed do protocolo

- **Dado** uma mensagem fora da ordem do ciclo de vida
- **Quando** a sessão a consome
- **Então** ela é rejeitada com erro tipado: antes do handshake
  (`NotHandshaked`), handshake duplicado (`AlreadyHandshaked`) e qualquer
  mensagem após shutdown (`AfterShutdown`)

#### AC-679 — Correlação exata de requests

- **Dado** requests registrados por id
- **Quando** chegam responses ou cancels
- **Então** somente ids pendentes são aceitos; id duplicado
  (`DuplicateRequest`), id desconhecido e replay de response já consumido
  (`UnknownRequest`) falham fechadamente

#### AC-680 — Isolamento de contexto por request

- **Dado** um request vinculado a projeto/sessão/trace
- **Quando** o response devolve contexto diferente
- **Então** a sessão rejeita com `ContextMismatch` sem consumir o request
  pendente

#### AC-681 — Limites, versão e capacidade fail-closed

- **Dado** mensagens malformadas, oversized ou em versão não suportada
- **Quando** validadas isoladamente ou na sessão
- **Então** payload acima do bound (`OversizedPayload`), handshake sem
  capabilities (`InvalidPayload`), schema/protocolo incompatível
  (`UnsupportedVersion`) e capacidade de pending excedida
  (`Backpressure`) falham fechadamente

#### AC-682 — Erro e cancelamento sem instrução executável

- **Dado** mensagens de erro e cancelamento
- **Quando** validadas e serializadas
- **Então** cancel carrega somente motivo bounded, erro carrega código e
  detalhe bounded sem segredo, sucesso não carrega erro, rejeição exige
  detalhe e nenhuma mensagem de controle embute instrução executável

## Fora de escopo

- Processo worker Python, transporte JSON-RPC e SDK (PR-113..PR-116).
- Registro de tools Python, execução de código arbitrário e telemetria.
- Persistência da sessão entre processos.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-629 | O contrato pode viver no `agent-protocol` sem depender de `agent-core` (sem ciclo de dependência). | confirmada | `worker.rs` usa somente ids, capability, envelope e serde existentes da própria crate. |
| ASM-630 | Budget/reservation é referenciado por identidade no runtime, não no contrato worker. | confirmada | O contrato transporta contexto (projeto/sessão/task/trace); reservation segue na camada runtime. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-618 | O handshake precisa negociar múltiplas versões no futuro? | respondida | Não na v1: o contrato aceita somente a versão vigente (`UnsupportedVersion` caso contrário). Negociação de range, se exigida pelo transporte (PR-114), entrará como nova pergunta com evidência própria. |
