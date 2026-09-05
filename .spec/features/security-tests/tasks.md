# Tasks: security tests

> feature: security-tests
> card: PR-260

## T-2100 — Threat regression manifest versionado e suíte fail-closed [em-andamento]

- Refs: US-2100, AC-2101, AC-2102, AC-2103, AC-2104, AC-2105, AC-2106, AC-2107
- Arquivos: `docs/security/threat-regression-manifest.json`,
  `tools/security/threat-regression.mjs`,
  `tools/security/threat-regression.spec.mjs`,
  `tools/security/README.md`,
  `.github/workflows/ci-security.yml`,
  `crates/security-core/tests/security_regression_contract.rs`,
  `.spec/features/security-tests/spec.md`,
  `.spec/features/security-tests/tasks.md`,
  `.github/workflows/onp-sdd-evidence.yml`
- Evidência local: `cargo test -p security-core --test security_regression_contract` PASS,
  `node --test tools/security/threat-regression.spec.mjs` PASS, ONP verify 7/7 ACs, actionlint PASS.
- O manifest TM-001..TM-007 é a única fonte canônica de TMs; runner exige
  AC-NNNN e test_id referenciados; nenhum resultado parcial é aceito.

## Suposições

- O manifest fica em `docs/security/threat-regression-manifest.json` e é a
  única fonte canônica de TM-NNN.
- Os runners Rust vivem em
  `crates/security-core/tests/security_regression_contract.rs` e os Node em
  `tools/security/threat-regression.spec.mjs`; nenhum dos dois afirma
  ausência de vulnerabilidade.
- O workflow `ci-security.yml` é `pull_request` + `push: main` e roda
  somente em `ubuntu-24.04`, sem privilégios de escrita no repositório.
- A execução do runner é bounded por tempo (≤10 min), por tamanho de
  output e por quantidade de TMs (≤128).
