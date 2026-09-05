# Tasks: audit logs

> feature: audit-logs
> card: PR-259
> status: em-andamento
> owner: agent-runtime

## T-2020 — Implementar `AuditLog` puro em `security-core`

- **Status:** em-andamento
- **Plano de card:** PR-259 (Add audit logs)
- **Referências:** US-2020, AC-2021, AC-2022, AC-2023, AC-2024, AC-2025.
- **Dependências anteriores:** PR-250, PR-252, PR-255, PR-256, PR-257, PR-258.

### Escopo desta task

- Adicionar `crates/security-core/src/audit.rs` com `AuditLog`, `AuditEvent`,
  `AuditPolicy`, `RedactedField`, `AuditSink`, `AuditError`, `AuditIntegrity`,
  `AuditExport`, `AuditQuery` e serialização determinística.
- Adicionar `crates/security-core/tests/audit_log_contract.rs` cobrindo
  AC-2021..AC-2025.
- Adicionar `docs/audit-logs.md` descrevendo boundary, sinks, redaction,
  integridade, retenção e limites.
- Adicionar step explícito `Verify audit logs` em
  `.github/workflows/onp-sdd-evidence.yml`.

### Não-escopo

- I/O de disco, rede, relógio real, secret store ou sink concreto.
- Persistência durável, SQL/index, forwarding SIEM, agregação por janela.
- Aprovação de ações baseada em auditoria.

### Critérios de aceite

- `AuditLog::record` retorna `AuditError` tipado em sink que falha.
- `verify_chain` classifica `Ok`, `Missing`, `OutOfOrder`, `HashMismatch`,
  `Broken` de forma explícita.
- `RedactedField::secret` aparece como `[REDACTED]` em `serialize`,
  `export`, `query` e diff.
- Retenção por duração, por capacidade e por escopo é respeitada e testada.
- Determinismo da serialização é testado por fixture.

### Riscos

- Esquecer de redigir um novo caminho de serialização → cobrir todo output em
  testes.
- Permitir que `record` ignore erro de sink → sink deve ser sempre avaliado e
  o erro propagado.
- Aceitar payloads não-bounded → validar `payload_bytes <= MAX_PAYLOAD`.

### Evidência

- `cargo test -p security-core --test audit_log_contract` PASS.
- `cargo clippy -p security-core --all-targets --locked --offline -- -D warnings` PASS.
- `CI=1 node tools/ci/run-onp-spec.mjs verify audit-logs` PASS.
- `node tools/run-feature-tests.mjs audit-logs` PASS.
