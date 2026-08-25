# Spec: Parallel invocation batch

> feature: parallel-invocation
> status: implementada

## História de usuário

### US-903 — Planejar fan-out paralelo bounded

Como boundary de grupo, quero aceitar um batch somente de invocations
preflightadas, para limitar fan-out/concurrency, preservar isolamento e
produzir join determinístico sem iniciar workers implicitamente.

#### AC-905 — Candidatos válidos formam batch ordenado

- **Dado** children independentes com cycle/depth `Pass`
- **Quando** o batch é preparado
- **Então** preserva a ordem de entrada e impõe limite de concurrency.

#### AC-906 — Gates inválidos nunca entram

- **Dado** preflight rejeitado, duplicate, fan-out excedido ou scope divergente
- **Quando** o batch é preparado
- **Então** falha fechado antes de qualquer execução.

#### AC-907 — Cancelamento e join são determinísticos

- **Dado** batch preparado
- **Quando** cancelado ou joined
- **Então** cancelamento é idempotente e outcomes são retornados na ordem do
  batch, sem órfãos.

## Fora de escopo

- Execução de provider/worker, scheduler unbounded, group budget final,
  moderator, rounds, synthesis e UI.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-908 | Sem `agent-orchestrator`, o boundary seguro desta fatia é um planner em `agent-core`. | confirmada | Nenhum worker é iniciado. |

## Perguntas em aberto

Nenhuma.
