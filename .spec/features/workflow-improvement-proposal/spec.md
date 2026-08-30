# Spec: workflow improvement proposal

> feature: workflow-improvement-proposal
> status: auditada

### US-1357 — Validated workflow diff

Como avaliador, quero uma proposal de workflow before/after com diff bounded e impacto explícito.

#### AC-1357 — Topology and compatibility

- **Dado** proposal válida sem ciclo, com estados compatíveis e referências declaradas.
- **Quando** criada.
- **Então** produz digest estável e preserva a versão ativa.
- **Dado** ciclo ou estado removido incompatível.
- **Quando** criada.
- **Então** é bloqueada.

### US-1358 — Capability boundary

Como sistema, quero bloquear capability privilegiada e budget delta sem policy explícita.

#### AC-1358 — Safe proposal only

- **Dado** novo nó privilegiado, subworkflow oculto ou budget escalation sem aprovação.
- **Quando** a proposal é criada.
- **Então** é bloqueada e não ativa o workflow.
- **Dado** diff válido.
- **Quando** consultado.
- **Então** permanece somente input de avaliação e rollback preserva a versão anterior.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Proposal de DAG bounded, deterministicamente identificada e reversível, sem mutação do workflow ativo.
