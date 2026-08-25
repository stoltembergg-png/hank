# Spec: Agent invocation protocol

> feature: agent-invocation-protocol
> status: implementada

## História de usuário

### US-881 — Transportar invocation com identidade e limites

Como orchestrator, quero um contrato provider-agnostic de invocation, para que
futuras delegações carreguem identidade, trace, budget, depth e contexto
allowlisted sem executar bypass ou compartilhar memória implícita.

#### AC-882 — Request válido é versionado e bounded

- **Dado** project/group/session/caller/callee/trace válidos, task e contexto
  `project://`
- **Quando** request é validado
- **Então** passa com schema version e estado `Pending`.

#### AC-883 — Identidade, budget, contexto e depth inválidos falham

- **Dado** campo ausente, budget zero, path externo ou depth acima do limite
- **Quando** request é validado
- **Então** falha fechado sem transporte.

#### AC-884 — Estados terminais são dados, não execução

- **Dado** response `Completed` ou `Cancelled`
- **Quando** contrato é validado
- **Então** estado permanece serializável e nenhuma delegation é disparada.

## Fora de escopo

- Delegation tool, graph, cycle detection, transport, provider, scheduler e UI.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-885 | Protocol types vivem em `agent-protocol` e não dependem de runtime/provider. | confirmada | Mantida separação arquitetural. |

## Perguntas em aberto

Nenhuma.
