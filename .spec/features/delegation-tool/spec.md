# Spec: Delegation tool

> feature: delegation-tool
> status: implementada

## História de usuário

### US-886 — Criar delegation pending com gates

Como agent caller, quero solicitar delegation apenas para membro validado,
para que a operação seja auditável, deduplicável e não execute bypass de
policy, budget, trace ou invocation protocol.

#### AC-887 — Target válido cria request Pending

- **Dado** sessão e caller/callee membros no mesmo projeto
- **Quando** delegation builder recebe task/context/budget bounded
- **Então** retorna `InvocationRequest` Pending e não chama worker/provider.

#### AC-888 — Target/task/context inválido é negado

- **Dado** target desconhecido, task oversized ou contexto inválido
- **Quando** builder é chamado
- **Então** retorna erro fail-closed sem criar execução.

#### AC-889 — Pending ledger deduplica e cancela

- **Dado** mesmo invocation ID
- **Quando** request é registrado ou cancelado repetidamente
- **Então** primeira operação é aplicada, retry é no-op e nenhum worker é chamado.

## Fora de escopo

- Invocation graph, cycle/depth enforcement adicional, parallel execution,
  provider, transport, moderator, synthesis e UI.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-890 | O boundary de tool existente é `tool-core`; não há crate orchestrator no workspace. | confirmada | Builder foi implementado em `tool-core`. |

## Perguntas em aberto

Nenhuma.
