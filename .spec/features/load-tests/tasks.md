# Tasks: load tests

> feature: load-tests
> card: PR-262

## T-2300 — Add bounded load contract [concluida]

- Refs: US-2300, AC-2301, AC-2302, AC-2303, AC-2304, AC-2305
- Arquivos: `.github/workflows/ci-load.yml`, `.github/workflows/onp-sdd-evidence.yml`, `.spec/features/load-tests/spec.md`, `.spec/features/load-tests/tasks.md`, `crates/test-support/src/load.rs`, `crates/test-support/src/lib.rs`, `crates/test-support/tests/load_contract.rs`, `docs/performance/load-manifest.json`, `docs/performance/load-tests.md`, `onpspec.config.json`, `tools/security/load-feature-tests.mjs`, `tools/security/load-runner.mjs`
- T-2300.1 — Perfis S/M/L, seed, warmup/repetitions e digest de fixture bounded.
- T-2300.2 — Modelo determinístico de admissão, fila, cancelamento e receipt.
- T-2300.3 — Contrato Rust com cinco testes AC-2301..AC-2305.
- T-2300.4 — Runner Node com watchdog, manifest validation e artifact sem timestamp.
- T-2300.5 — Workflow dedicado e step ONP executam a lane com permissões somente de leitura.
- T-2300.6 — Documentação e escopo deixam explícita a ausência de métricas de host e tráfego de produção.
- Evidência: `cargo fmt`, `cargo check -p test-support --locked --offline`, `cargo clippy -p test-support --all-targets --locked --offline -- -D warnings`, contrato Rust 5/5, runner PASS, TAP 5/5 e `verify load-tests` 5/5.
