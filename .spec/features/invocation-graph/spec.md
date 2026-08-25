# Spec: Invocation graph

> feature: invocation-graph
> status: implementada

## História de usuário

### US-891 — Registrar ancestry de invocations

Como orchestrator, quero registrar requests pending em um grafo bounded, para
que parentagem, dedupe, cancelamento e isolamento de projeto sejam verificáveis
antes de qualquer execução.

#### AC-892 — Parent válido registra nó pending

- **Dado** request válido e parent no mesmo projeto
- **Quando** o grafo registra o request
- **Então** o nó fica pending e conserva ancestry.

#### AC-893 — Parent ausente, duplicate e scope mismatch falham

- **Dado** parent inexistente, ID repetido ou projeto divergente
- **Quando** o request é registrado
- **Então** falha fechado.

#### AC-894 — Cancelamento é idempotente e não executa

- **Dado** nó pending
- **Quando** cancelado repetidamente
- **Então** primeira operação marca cancelled e retry não altera o grafo.

## Fora de escopo

- Cycle detection adicional, scheduler, parallelism, provider, transport e UI.

## Perguntas em aberto

Nenhuma.
