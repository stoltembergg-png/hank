# Spec: Group budget accounting

> feature: group-budget
> status: implementada

## História de usuário

### US-908 — Reservar e reconciliar budget compartilhado

Como grupo multi-agent, quero uma conta compartilhada por project/group, para
que reservations sejam atômicas, retries não cobrem duas vezes e cancelamentos
liberem o não consumido.

#### AC-910 — Reserva atômica e isolamento por invocation

- **Dado** budget válido e invocation ID novo
- **Quando** uma reserva é criada
- **Então** tokens/cost/parallel são contabilizados contra o grupo e retry é
  deduplicado.

#### AC-911 — Uso real reconcilia sem double charge

- **Dado** reservation ativa
- **Quando** commit informa uso real
- **Então** a reserva é removida e somente o uso real vira consumo; retry falha.

#### AC-912 — Cancelamento devolve reserva

- **Dado** reservation pending
- **Quando** refund é chamado
- **Então** disponibilidade retorna e refund repetido falha sem mutar outra
  reservation.

## Fora de escopo

- Pricing provider, UI, scheduler, telemetry externa e alteração do budget
  global.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-913 | `BudgetAccount` existente é a primitive de reserva e reconciliation. | confirmada | `GroupBudget` apenas aplica project/group/invocation boundary. |

## Perguntas em aberto

Nenhuma.
