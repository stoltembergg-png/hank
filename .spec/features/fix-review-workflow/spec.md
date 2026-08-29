# Spec: fix-review workflow

> feature: fix-review-workflow
> status: auditada

## Contexto

PR-216 define um plano puro e bounded para transformar findings blockers em tarefas de correção no mesmo vínculo de projeto/tarefa/repositório/worktree/branch. Adapters externos criam a tarefa e o worktree; este domínio apenas valida e propõe.

## Histórias

### US-1345 — Correction mapping

Como sistema de revisão, quero preservar o mapping original e invalidar evidência antiga após um novo commit.

#### AC-1345 — Mapping and supersession

- **Dado** finding blocker e mapping válido.
- **Quando** um plano de correção é criado.
- **Então** a tarefa proposta preserva o mapping e referencia a evidência superseded.
- **Dado** commit diferente do finding.
- **Quando** evidência antiga é avaliada.
- **Então** ela é rejeitada como stale.

### US-1346 — Bounded retry

Como sistema de revisão, quero limitar ciclos de correção e escalar ao atingir o teto.

#### AC-1346 — Cycle cap

- **Dado** ciclo abaixo do limite.
- **Quando** a correção é planejada.
- **Então** o próximo ciclo é permitido.
- **Dado** ciclo no limite.
- **Quando** nova correção é solicitada.
- **Então** o plano retorna escalada, sem retry infinito.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Contrato puro, bounded, idempotente por fingerprint, sem criação de task/worktree, sem Git/rede/credencial e com verify/audit ONP.
