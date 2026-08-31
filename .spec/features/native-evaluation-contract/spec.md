# Spec: native evaluation contract

> feature: native-evaluation-contract
> status: auditada

## Contexto

O Harness precisa comparar candidatos com uma referência congelada sem deixar
que o candidato escolha a fixture, o scorer ou a autoridade da avaliação. O
contrato é dev-only, provider-neutral e limitado a identidades, descritores e
evidência bounded; runner, corpus, persistência, UI e efeitos externos ficam
em cards posteriores.

### US-1435 — Definir um caso de avaliação versionado

Como operador do Harness, quero um caso determinístico vinculado a
`project/run/trace`, fixture, scorer, policy, schema e classe de modelo.

#### AC-1435 — Identidade e replay são determinísticos

- **Dado** um caso com identidade tipada, fixture e scorer versionados.
- **Quando** o caso é construído e serializado novamente.
- **Então** o wire contract faz round-trip, mantém a identidade exata e o
  fingerprint permanece estável, sem armazenar prompt ou raciocínio privado.

### US-1436 — Exigir autoridade, terminal e partição

Como gate de avaliação, quero que a política de efeitos, o terminal esperado e
o metadata de training/holdout sejam obrigatórios.

#### AC-1436 — Metadata incompleto falha fechado

- **Dado** um caso sem authority, terminal esperado ou marker de holdout.
- **Quando** o contrato é validado.
- **Então** ele é rejeitado sem criar um plano de execução.

### US-1437 — Versionar métricas estruturadas

Como comparador, quero uma lista versionada de métricas conhecidas, com tipo,
direção e limites explícitos.

#### AC-1437 — Métrica desconhecida ou campo sensível não entra no wire

- **Dado** payload com métrica desconhecida ou campo de segredo.
- **Quando** o payload é desserializado.
- **Então** a boundary rejeita o payload; somente métricas estruturadas e
  bounded são aceitas.

### US-1438 — Impedir efeitos não autorizados

Como mantenedor de segurança, quero que fixtures não determinísticas e efeitos
externos não tenham autoridade de avaliação.

#### AC-1438 — Fixture insegura e write externo falham fechado

- **Dado** fixture não determinística ou efeito externo declarado.
- **Quando** o caso é validado.
- **Então** a avaliação é rejeitada sem chamar ferramenta, processo, rede ou
  filesystem de produção.

### US-1439 — Vincular relatório à referência

Como consumidor de evidência, quero um relatório bounded que só seja aceito
quando seus IDs, métricas, artifacts e digests coincidirem com o caso.

#### AC-1439 — Baseline report não aceita identidade ou métrica divergente

- **Dado** um relatório de baseline e seu caso de avaliação.
- **Quando** project/run/trace, métricas ou artefatos divergem.
- **Então** o relatório é rejeitado e nunca concede ativação.

### US-1440 — Preservar cancelamento e ausência de prova

Como operador, quero distinguir cancelamento e `NO_PROOF` de sucesso sem
permitir promoção implícita.

#### AC-1440 — Cancelado e sem prova permanecem não promocionáveis

- **Dado** um relatório cancelado ou sem prova.
- **Quando** ele é validado contra o caso.
- **Então** o estado permanece explícito, bounded e não promocionável.

## Fora de escopo

- execução de runner, corpus de benchmarks ou comparação baseline/candidate;
- providers, ferramentas, filesystem, rede, persistência, UI ou secrets;
- seleção de casos pelo candidato e qualquer ativação automática.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Contrato `EvaluationCase`, schema de métricas, autoridade, holdout e
`BaselineReport` versionados, bounded e fail-closed; testes positivos e
negativos, documentação e verify/audit ONP passando no SHA exato.
