# tools/security

Tooling pura para as lanes de regressão de segurança.

- `threat-regression.mjs` — runner da matriz de ameaças da PR-260; valida o
  manifest, executa as verificações negativas e produz um receipt bounded.
- `security-feature-tests.mjs` — wrapper TAP Rust da PR-260.
- `fuzz-runner.mjs` — valida o manifest FT-001..FT-007, executa exatamente os
  10 testes do contrato Rust e grava um receipt determinístico da PR-261.
- `fuzz-feature-tests.mjs` — wrapper TAP usado pelo ONP; verifica nomes e
  resultados individuais antes de emitir as tags `@spec`.
- `fuzz-tests.spec.mjs` — contratos Node do manifest, digest e relatório.

A lane de fuzz usa somente corpus sintético/redacted e não contata providers,
serviços de produção ou stores de credenciais. CI executa `cargo fetch --locked`
uma vez e depois usa `CARGO_NET_OFFLINE=true`/`--offline` para o contrato.
Receipts não preservam stdout, stderr, timestamps ou valores sensíveis; os
identificadores de credencial usados em testes são sempre `[REDACTED]`.
