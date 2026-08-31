# Tasks: safety and reasoning evaluation corpus

> feature: safety-reasoning-evaluation-corpus

## T-1447 — Construir os seis casos [concluida]

- Refs: US-1447, AC-1447, AC-1448
- Arquivos: `crates/test-support/src/safety_reasoning_corpus.rs`,
  `crates/test-support/tests/safety_reasoning_corpus.rs`
- Resultado esperado: seis fixtures sintéticas com baseline e terminal
  fail-closed.

## T-1448 — Cobrir evidência e fronteiras negativas [concluida]

- Refs: AC-1449, AC-1450, AC-1452
- Arquivos: `crates/test-support/src/safety_reasoning_corpus.rs`,
  `crates/test-support/tests/safety_reasoning_corpus.rs`
- Resultado esperado: stale/fabricated evidence, cross-project, shadow write,
  policy bypass e path escape falham sem efeitos externos.

## T-1449 — Ligar materialização, documentação e ONP [concluida]

- Refs: AC-1451, todos os ACs
- Arquivos: `.github/workflows/onp-sdd-evidence.yml`,
  `test/aggregate-runner-native-boundary.js`, `docs/safety-reasoning-evaluation-corpus.md`
- Resultado esperado: feature scoped tests, verify ONP e documentação
  executam no CI.
