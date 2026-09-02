# Tasks: native evaluation corpus

> feature: native-evaluation-corpus

## T-1441 — Construir corpus core [concluida]

- Refs: US-1441, AC-1441, AC-1442
- Arquivos: `crates/test-support/src/evaluation_corpus.rs`,
  `crates/test-support/tests/evaluation_corpus.rs`
- Resultado: seis cases sintéticos com baseline report, metrics e artifacts.

## T-1442 — Ligar materialização e negativas [concluida]

- Refs: US-1443, US-1444, US-1445, US-1446
- Arquivos: `crates/test-support/src/evaluation_corpus.rs`,
  `crates/test-support/tests/evaluation_corpus.rs`
- Resultado: digest de fixture verificado em workspace temporário, efeitos
  externos proibidos, terminais blocked/cancelled e evidência stale rejeitada.

## T-1443 — Ligar verificação SDD/ONP [concluida]

- Refs: todos os ACs
- Arquivos: `.github/workflows/onp-sdd-evidence.yml`,
  `test/aggregate-runner-native-boundary.js`
- Resultado: feature incluída no verify ONP e no runner agregado.
