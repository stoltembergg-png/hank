# Spec: automated evaluation

> feature: automated-evaluation
> status: auditada

### US-1361 — Comparable baseline and candidate report

Como avaliador, quero comparar baseline e candidate nas mesmas fixtures determinísticas.

#### AC-1361 — Exact identity and bounded evidence

- **Dado** manifest com baseline, candidate, SHA, fixtures e seed exatos.
- **Quando** a avaliação é criada.
- **Então** produz relatório bounded com métricas de qualidade, segurança, custo e latência.
- **Dado** SHA incorreto, baseline/fixture ausente, timeout ou skip.
- **Quando** a avaliação é criada.
- **Então** o estado é `Unknown` e não autoriza aprovação.

### US-1362 — Deterministic failure propagation

Como sistema, quero garantir que regressão ou recurso excedido bloqueie a evidência.

#### AC-1362 — Evidence is not rollout authority

- **Dado** resultado com regressão além do threshold ou recurso excedido.
- **Quando** o relatório é finalizado.
- **Então** o estado é `Fail`/`Unknown`, sem ativação automática.
- **Dado** execução repetida com o mesmo manifest e seed.
- **Quando** comparada.
- **Então** a identidade do relatório é estável.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Manifest e relatório determinísticos, bounded, redacted e vinculados à identidade exata; evaluator não ativa rollout.
