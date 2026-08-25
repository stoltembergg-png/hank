# Spec: AgentGroup repository

> feature: agent-group-repository
> status: implementada

## História de usuário

### US-859 — Persistir AgentGroup com escopo, revisão e rollback

Como plataforma multi-agent, quero persistir grupos com transações e revisão
otimista, para que restart, archive e retry não percam isolamento ou policy.

#### AC-861 — Create/get são project-scoped

- **Dado** grupo válido e projeto existente
- **Quando** é criado e lido
- **Então** a revisão inicial é 1 e outro projeto não encontra o grupo.

#### AC-862 — Duplicate/stale update falham fechados

- **Dado** grupo já persistido ou revisão incorreta
- **Quando** create/archive é repetido com conflito
- **Então** retorna erro sem sobrescrever estado.

#### AC-863 — Archive é idempotente e preserva policy

- **Dado** grupo válido
- **Quando** archive é aplicado e repetido
- **Então** a primeira operação incrementa revisão e a repetição não incrementa.

## Fora de escopo

- Membership mutation, sessions, invocation, delegation, UI e execução.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-864 | O repository vive em `agent-runtime` até existir crate orchestrator aprovado. | confirmada | Mantida arquitetura existente e migrations SQLite. |

## Perguntas em aberto

Nenhuma.
