# Spec: regression evaluation

> feature: regression-evaluation
> status: auditada

### US-1363 — Versioned regression corpus

Como avaliador, quero selecionar um corpus de regressão por impacto declarado e identidade exata.

#### AC-1363 — Corpus and applicability

- **Dado** corpus íntegro, baseline/candidate e impacto conhecido.
- **Quando** o gate é executado.
- **Então** produz resultado bounded e comparável.
- **Dado** fixture removida ou classificador desconhecido.
- **Quando** o gate é executado.
- **Então** retorna `NoGo`.

### US-1364 — Fail-closed regression gate

Como sistema, quero impedir aprovação quando a evidência é incompleta ou crítica.

#### AC-1364 — Critical and stale evidence

- **Dado** skip, no-run, identidade stale ou regressão crítica.
- **Quando** o gate é finalizado.
- **Então** retorna `NoGo` e não autoriza rollout.
- **Dado** corpus válido sem regressão.
- **Quando** repetido.
- **Então** o fingerprint permanece estável.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Gate de regressão bounded, versionado e fail-closed, sem alteração de testes existentes ou rollout.
