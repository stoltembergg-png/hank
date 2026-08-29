# Spec: review workflow

> feature: review-workflow
> status: auditada

## Contexto

PR-214 agrega resultados bounded de reviewer, QA, security e architecture em um relatório advisory vinculado ao mesmo task, repository, worktree, branch, commit, tree e policy.

## Histórias

### US-1340 — Review fail-closed

Como sistema de revisão, quero rejeitar evidência ausente, stale, skipped, malformed ou de identidade divergente.

#### AC-1340 — Evidência e identidade

- **Dado** relatório com identidade exata e evidência PASS/FAIL completa.
- **Quando** o relatório é agregado.
- **Então** findings são bounded e o estado é determinístico.
- **Dado** SHA/tree/policy/repository/task divergente ou evidência ausente/stale/skipped/malformed.
- **Quando** o relatório é agregado.
- **Então** o estado é `blocked` e a evidência é desconhecida.

### US-1341 — Autoridade humana

Como sistema de segurança, quero que texto `approved` de uma IA não altere o estado de revisão.

#### AC-1341 — Blocker e escalonamento

- **Dado** qualquer blocker ou evidência desconhecida.
- **Quando** a revisão é calculada.
- **Então** o resultado permanece bloqueado/advisory e nunca autoriza ready, approval ou merge.
- **Dado** findings sem blocker e evidência completa.
- **Quando** a revisão é calculada.
- **Então** o relatório é advisory, exigindo decisão humana externa.

### US-1342 — Invalidação

Como sistema de evidência, quero invalidar o relatório quando o commit ou policy mudar.

#### AC-1342 — Stale report

- **Dado** relatório produzido para outra identidade.
- **Quando** comparado ao contexto atual.
- **Então** é rejeitado como stale, sem reaproveitamento de estado verde.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Contrato puro, testes positivos/negativos, documentação e verify ONP; nenhuma chamada GitHub/Git/rede/filesystem/processo e nenhuma autoridade de merge.
