# Tasks: audit logs

> feature: audit-logs
> card: PR-259

## T-2020 — Implementar `AuditLog` puro em `security-core` [em-andamento]

- Refs: US-2020, AC-2021, AC-2022, AC-2023, AC-2024, AC-2025
- Arquivos: `crates/security-core/src/audit.rs`, `crates/security-core/src/lib.rs`,
  `crates/security-core/Cargo.toml`, `crates/security-core/tests/audit_log_contract.rs`,
  `docs/audit-logs.md`, `.spec/features/audit-logs/spec.md`,
  `.spec/features/audit-logs/tasks.md`, `.github/workflows/onp-sdd-evidence.yml`,
  `apps/desktop/src-tauri/Cargo.lock`
- Evidência local: `cargo test -p security-core --locked --offline` — 13/13
  audit_log_contract + 7/7 rate_limit + 3/3 security_profile + 2/2 outros = 25/25.
- `cargo clippy -p security-core --all-targets --locked --offline -- -D warnings` PASS.
- `cargo fmt -p security-core -- --check` PASS.
- `CI=1 node tools/ci/run-onp-spec.mjs verify audit-logs` — 5/5 ACs com prova PASS.
- `node tools/run-feature-tests.mjs audit-logs` — 1 comando / 13 testes, PASS.
- O contrato implementa `AuditPolicy` (capacidade, retenção, revisão), `AuditEvent`
  (actor/resource/policy_revision/SHA256/sequência/hash), `AuditSink` (trait com
  `InMemorySink`), `AuditQuery` (filtros bounded), `AuditIntegrity` (Ok/Missing/
  OutOfOrder/HashMismatch/Broken), redaction de `RedactedField::Secret` para
  `[REDACTED]` antes de qualquer serialização, e propagação tipada de erro
  quando o sink falha. Sem I/O de disco, sem rede, sem relógio real.
- Audit global: feature `audit-logs` contribui 0 erros e 6 avisos (5x `PROVA_FRACA`
  sobre `reporter=exitcode` — o mesmo padrão já documentado no baseline do
  repositório como `NO_PROOF`; o reporter `exitcode` é o formato atual do
  `cargo test`).

## Suposições

- A policy e seu ledger permanecem em memória nesta fatia; persistência, rede
  distribuída, secret store e forwarding para SIEM não são inferidos por
  testes de contrato e vivem em adapters fora de `security-core`.
- O `audit-sink` concreto (storage-core, remote-core, agent-runtime,
  telemetry-core) será introduzido em card subsequente; este card entrega
  apenas o contrato e o ledger puro.
