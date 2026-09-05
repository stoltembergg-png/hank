# tests/fuzz-tests — Fuzz harness (PR-261)

Pura tooling para a camada de regressão de fuzz (PR-261).
- `fuzz-runner.mjs` — runner Node, sem I/O fora do workspace, que
  carrega o manifest, valida schema, executa o contrato Rust
  (`test-support::fuzz_contract`) e produz `security/reports/fuzz.json`
  com `tree_sha`, `head_sha`, `runner_digest` e o resumo do contrato.
- `fuzz-tests.spec.mjs` — suíte `node --test` que valida o contrato do
  runner e da matriz. Cada teste carrega tag `@spec:AC-22NN` para o ONP.
- O contrato Rust está em `crates/test-support/tests/fuzz_contract.rs`
  e os alvos em `crates/test-support/src/fuzz_targets.rs`.

Os runners são executados em `ubuntu-24.04` pelo workflow
`.github/workflows/ci-fuzz.yml`. O runner nunca afirma ausência de
vulnerabilidade; ele apenas confirma que o manifest e a suíte
permanecem coerentes.
