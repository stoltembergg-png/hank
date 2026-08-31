# Spec: planning reconciliation

> feature: planning-reconciliation
> status: auditada

## Contexto

PR-390 materializa a reconciliação bounded entre PlannerDraft e findings de
reviewers. O contrato é provider-neutral, preserva provenance e nunca concede
autoridade de execução, aprovação ou merge.

### US-1403 — Reconcile bounded planning findings

Como sistema de planejamento, quero transformar findings de reviewers em um
FinalPlan determinístico, auditável e limitado ao mesmo projeto, run e trace.

#### AC-1403 — Disposition matrix and evidence

- **Dado** finding com severidade e evidência compatíveis.
- **Quando** a reconciliação é executada.
- **Então** a disposição é determinística e findings high/critical exigem
  evidência verificada.

#### AC-1404 — Deduplication preserves provenance

- **Dado** findings equivalentes ou conflitantes de reviewers distintos.
- **Quando** agrupados.
- **Então** a decisão é consolidada sem apagar IDs, reviewers ou evidências
  de origem; divergência permanece observável.

#### AC-1405 — Policy and product conflicts escalate

- **Dado** conflito de policy/produto não resolvido.
- **Quando** o plano final é calculado.
- **Então** o status é `HUMAN_REQUIRED` e não há caminho implícito de execução.

#### AC-1406 — Bounds and self-approval fail closed

- **Dado** input fora dos limites, round acima do máximo ou identidade de
  planner/judge/reviewer sobreposta.
- **Quando** validado.
- **Então** a reconciliação falha fechada.

#### AC-1407 — FinalPlan is versioned and non-authoritative

- **Dado** FinalPlan serializado.
- **Quando** lido por outro componente.
- **Então** schema, identidade e limites são preservados e o plano não pode
  aprovar, fazer merge ou executar.

#### AC-1408 — Cancellation, idempotency and reopen

- **Dado** mesma entrada, cancelamento ou rollback.
- **Quando** processado novamente.
- **Então** a saída é idempotente, o cancelamento não executa efeitos e o
  rollback cria um novo draft sem mutar o plano imutável anterior.

## Fora de escopo

- execução de plano, provider, scheduler, UI, persistência, GitHub, Git,
  filesystem, rede, secrets ou autoridade humana implementada;
- binding de evidência externa, reservado para a PR-391.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Contrato puro, bounded, versionado, idempotente e fail-closed, com testes de
contrato positivos e negativos, documentação e verify/audit ONP.
