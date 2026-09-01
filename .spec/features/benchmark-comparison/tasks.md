# Tasks: skill version benchmark comparison

> feature: benchmark-comparison

## T-1491 — Implementar comparação nativa bounded [concluida]

- Refs: US-1491, AC-1491, AC-1492, AC-1493, AC-1494
- Arquivos: `crates/test-support/src/benchmark_comparison.rs`,
  `crates/test-support/src/evaluation_runner.rs`,
  `crates/test-support/tests/benchmark_comparison.rs`
- Resultado: comparator canônico com deltas training/holdout, thresholds,
  identidade de runs e artifact de revisão independente.

## T-1492 — Integrar contrato ONP e runbook [concluida]

- Refs: AC-1491, AC-1492, AC-1493, AC-1494
- Arquivos: `.github/workflows/onp-sdd-evidence.yml`,
  `test/aggregate-runner-native-boundary.js`,
  `docs/benchmark-comparison.md`
- Resultado: verify/audit da feature e documentação operacional bounded.
