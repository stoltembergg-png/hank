# Tasks: native evaluation runner

> feature: native-evaluation-runner

## T-1461 — Definir contrato executável e negativas [concluida]

- Refs: US-1461, US-1462, US-1463, US-1464
- Arquivos: `crates/test-support/src/evaluation_runner.rs`,
  `crates/test-support/tests/evaluation_runner.rs`
- Resultado: runner bounded, ambiente comparável, replay idempotente e erros
  fail-closed cobertos por testes positivos e negativos.

## T-1462 — Integrar verificação SDD/ONP e runbook [concluida]

- Refs: AC-1471, AC-1472, AC-1473, AC-1474
- Arquivos: `.github/workflows/onp-sdd-evidence.yml`,
  `test/aggregate-runner-native-boundary.js`,
  `docs/native-evaluation-runner.md`
- Resultado: feature runner, verify ONP e documentação operacional.

## T-1463 — Identity e digest auxiliares do corpus [concluida]

- Refs: AC-1471, AC-1472
- Arquivos: `crates/test-support/src/digest.rs`,
  `crates/test-support/src/evaluation_corpus.rs`
- Resultado: digest determinístico de identidade de ambiente e baselines.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.