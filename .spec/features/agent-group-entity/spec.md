# Spec: AgentGroup entity

> feature: agent-group-entity
> status: implementada

## História de usuário

### US-851 — Modelar grupo multi-agent bounded e project-scoped

Como plataforma multi-agent, quero uma entidade `AgentGroup` versionada e não
executável, para carregar membros, políticas de turnos, limites, orçamento,
contexto não confiável e lifecycle sem permitir cross-project access.

#### AC-855 — Grupo válido mantém invariantes de projeto e limites

- **Dado** um projeto, owner, trace, budget, limites e contexto `project://`
- **Quando** o grupo é validado
- **Então** a entidade é aceita em lifecycle `Draft`.

#### AC-856 — Membro, limite ou contexto inválido falha fechado

- **Dado** membro sem binding no projeto, limite zero ou referência fora de
  `project://`
- **Quando** o grupo é validado
- **Então** a validação rejeita sem qualquer execução ou acesso externo.

#### AC-857 — Lifecycle archive/activate é bounded e idempotente

- **Dado** grupo arquivado ou grupo sem versão pinned
- **Quando** lifecycle é solicitado
- **Então** archive repetido não muda estado e activation sem pin é rejeitada.

## Fora de escopo

- Repository, membership mutation, invocation, delegation, parallel execution,
  UI, moderator implementation e estado global mutável.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-858 | Binding de projeto do membro deve ser materializado na entidade antes do repository. | confirmada | `member_projects` torna a prova de isolamento explícita. |

## Perguntas em aberto

Nenhuma.
