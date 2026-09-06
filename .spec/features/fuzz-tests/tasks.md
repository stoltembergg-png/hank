# Tasks: fuzz tests

> feature: fuzz-tests
> card: PR-261

## T-2200 — Add bounded fuzz harness [concluida]

- Refs: US-2200, AC-2201, AC-2202, AC-2203, AC-2204, AC-2205, AC-2206, AC-2207
- Arquivos: `.github/workflows/ci-fuzz.yml`, `.github/workflows/onp-sdd-evidence.yml`, `.spec/features/fuzz-tests/spec.md`, `.spec/features/fuzz-tests/tasks.md`, `crates/test-support/src/fuzz.rs`, `crates/test-support/src/fuzz_targets.rs`, `crates/test-support/src/lib.rs`, `crates/test-support/tests/fuzz_contract.rs`, `docs/fuzz-tests.md`, `docs/security/fuzz-manifest.json`, `onpspec.config.json`, `tools/security/README.md`, `tools/security/fuzz-feature-tests.mjs`, `tools/security/fuzz-runner.mjs`, `tools/security/fuzz-tests.spec.mjs`
- T-2200.1 — Harness bounded com `FuzzTarget`, `FuzzHarness`, relatórios, digest, replay e verificação de corpus.
- T-2200.2 — Sete targets FT-001..FT-007 registrados e exercitados por corpus sintético.
- T-2200.3 — Contrato Rust com 10 testes: sete ACs e três regressões.
- T-2200.4 — Runner Node valida manifest, executa contrato Rust e produz `security/reports/fuzz.json`.
- T-2200.5 — Workflow dedicado e step ONP executam a lane com permissões somente de leitura.
- T-2200.6 — Documentação, manifest e configuração ONP mantêm a rastreabilidade da feature.
- Evidência: `cargo fmt`, `cargo clippy -p test-support --all-targets --locked --offline`, contrato Rust 10/10, Node 7/7, `verify fuzz-tests` 7/7 e actionlint PASS.
