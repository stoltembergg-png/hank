# Spec: AgentGroup membership

> feature: agent-group-membership
> status: implementada

## História de usuário

### US-865 — Gerenciar membership scoped e rollbackable

Como mantenedor de grupo, quero adicionar/remover agentes com role e actor
explícitos, para impedir cross-project access, escalada de permissão e
membership duplicada.

#### AC-866 — Membro acessível é adicionado uma vez

- **Dado** actor owner/moderator, agente no mesmo projeto e role bounded
- **Quando** add é solicitado
- **Então** membership é adicionada uma vez com role e project binding.

#### AC-867 — Escopo e autorização são obrigatórios

- **Dado** projeto diferente ou actor sem permissão
- **Quando** add é solicitado
- **Então** operação é negada sem alteração.

#### AC-868 — Remove e restauração usam snapshot bounded

- **Dado** membership existente e snapshot válido
- **Quando** remove/restore é solicitado
- **Então** membro owner não é removido e snapshot restaura o conjunto validado.

## Fora de escopo

- Repository próprio, sessions, invocation, mention parser, UI e execução.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-869 | Membership pura pode ser embutida na entidade antes da persistência especializada. | confirmada | Snapshot faz parte do contrato versionado de AgentGroup. |

## Perguntas em aberto

Nenhuma.
