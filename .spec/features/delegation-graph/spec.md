# Spec: Delegation graph view contract

> feature: delegation-graph
> status: implementada

## História de usuário

### US-939 — Projetar grafo de delegation bounded

Como usuário do group chat, quero visualizar ancestry, status, depth, budget e
razões de deny sem alterar o grafo nem executar delegations.

#### AC-940 — Ancestry acíclica tem nodes/edges determinísticos

- **Dado** eventos parent/child do mesmo project/session
- **Quando** são aplicados
- **Então** nodes e edges aparecem em ordem determinística com IDs estáveis.

#### AC-941 — Denials são visíveis e read-only

- **Dado** cycle/depth/budget denial
- **Quando** o evento é renderizado
- **Então** a razão permanece visível e nenhum cancel/approve/execute é
  disparado pelo projection.

#### AC-942 — Dedupe, isolamento e limites são fail-closed

- **Dado** evento duplicado, foreign scope, parent desconhecido ou label com
  markup
- **Quando** chega ao store
- **Então** não duplica, não cruza projeto, não inventa edge e escapa/trunca
  label.

## Fora de escopo

- alterar InvocationGraph, approve/execute delegation, SQLite e history ilimitada.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-943 | API/event bridge fornecerá parent IDs válidos quando houver ancestry. | confirmada | O projection rejeita parent desconhecido e não infere edges. |

## Perguntas em aberto

Nenhuma.
