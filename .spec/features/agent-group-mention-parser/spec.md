# Spec: AgentGroup mention parser

> feature: agent-group-mention-parser
> status: implementada

## História de usuário

### US-876 — Resolver menções como referências tipadas

Como sessão multi-agent, quero reconhecer menções explícitas contra o snapshot
de membership, para que texto não invoque identidade inexistente ou capability.

#### AC-877 — Menção exata resolve e deduplica

- **Dado** member acessível e sintaxe `@agent:<typed-id>`
- **Quando** parser recebe texto
- **Então** retorna um target tipado uma vez, sem solicitar invocation.

#### AC-878 — Input desconhecido, cross-project ou grande falha fechado

- **Dado** target desconhecido, membership de outro projeto ou input acima do
  limite
- **Quando** parser resolve menções
- **Então** retorna erro bounded sem acesso externo.

#### AC-879 — Texto comum não tem side effect

- **Dado** texto sem menção válida ou sintaxe incompleta
- **Quando** parser processa
- **Então** retorna texto sem target e `invocation_requested=false`.

## Fora de escopo

- Invocation, delegation, graph, parallel execution, persistence e UI.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-880 | A sintaxe inicial usa IDs tipados `@agent:<id>`. | confirmada | Ambiguidade nominal fica para evolução posterior. |

## Perguntas em aberto

Nenhuma.
