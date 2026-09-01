# Tasks: native evaluation runner

> feature: native-evaluation-runner

## T-1453 — Definir contrato executável e negativas [concluida]

- Refs: US-1453, US-1454, US-1455, US-1456
- Arquivos: `crates/test-support/src/evaluation_runner.rs`,
  `crates/test-support/tests/evaluation_runner.rs`
- Resultado: runner bounded, ambiente comparável, replay idempotente e erros
  fail-closed cobertos por testes positivos e negativos.

## T-1454 — Integrar verificação SDD/ONP e runbook [concluida]

- Refs: todos os ACs
- Arquivos: `.github/workflows/onp-sdd-evidence.yml`,
  `test/aggregate-runner-native-boundary.js`,
  `docs/native-evaluation-runner.md`
- Resultado: feature runner, verify ONP e documentação operacional.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.
