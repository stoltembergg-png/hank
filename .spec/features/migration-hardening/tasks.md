# Tasks: migration hardening

> feature: migration-hardening

## T-1800 — Implementar manifesto e gate de migration [em-andamento]

- Refs: US-1800, AC-1801, AC-1802, AC-1803, AC-1804, AC-1805, AC-1806
- Arquivos: `crates/agent-runtime/src/migration_hardening.rs`,
  `crates/agent-runtime/src/lib.rs`, `crates/agent-runtime/src/restore.rs`,
  `crates/agent-runtime/tests/migration_hardening_contract.rs`
- Limites: o runner usa migrations existentes e não adiciona migration de produto nem
  executa downgrade, secret migration, crash real ou disk-full real.

## T-1801 — Registrar política, evidência ONP e runbook [em-andamento]

- Refs: AC-1801, AC-1802, AC-1803, AC-1804, AC-1805, AC-1806
- Arquivos: `docs/migration-hardening.md`, `.github/workflows/onp-sdd-evidence.yml`
- Evidência pendente até os gates do branch e do PR executarem no SHA final.
