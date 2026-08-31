# Tasks: native evaluation contract

> feature: native-evaluation-contract

## T-1435 — Implementar contrato nativo de avaliação [concluida]

- Refs: US-1435, AC-1435, AC-1436
- Arquivos: `crates/test-support/src/evaluation.rs`,
  `crates/test-support/src/lib.rs`,
  `crates/test-support/tests/evaluation_contract.rs`
- Resultado: caso versionado, tipado, determinístico, scoped e com política
  explícita de cancelamento/idempotência.

## T-1436 — Implementar métricas, evidência e relatório [concluida]

- Refs: US-1437, US-1438, US-1439, US-1440
- Arquivos: `crates/test-support/src/evaluation.rs`,
  `crates/test-support/tests/evaluation_contract.rs`,
  `docs/native-evaluation-contract.md`
- Resultado: metric schema conhecido, fixture/scorer descriptors, holdout
  marker, evidência de SHA/tree/policy/schema/fixture/environment/artifacts e
  relatório baseline somente evidencial.

## T-1437 — Ligar verificação SDD/ONP [concluida]

- Refs: todos os ACs
- Arquivos: `.github/workflows/onp-sdd-evidence.yml`,
  `test/aggregate-runner-native-boundary.js`
- Resultado: a feature é verificada pelo workflow e pelo runner agregado.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.
