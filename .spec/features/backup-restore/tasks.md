# Tasks: backup restore

> feature: backup-restore

## T-1700 — Implementar restore SQLite staged e idempotente [em-andamento]

- Refs: US-1700, AC-1701, AC-1702, AC-1703, AC-1704, AC-1705
- Arquivos: `crates/agent-runtime/src/restore.rs`, `crates/agent-runtime/src/backup.rs`,
  `crates/agent-runtime/src/lib.rs`, `crates/agent-runtime/tests/database_restore_contract.rs`,
  `docs/database-restore.md`
- Evidência local: contrato focal de restore, incluindo clean/upgrade, dry-run,
  incompatibilidade, digest, retry, lock, allowlist, symlink e limite.
- Limites honestos: não há claim de execução de restore produtivo, coordenação de
  writers da aplicação, kill real de processo ou simulação real de disk-full.

## T-1701 — Registrar contrato ONP e runbook operacional [em-andamento]

- Refs: AC-1701, AC-1702, AC-1703, AC-1704, AC-1705
- Arquivos: `.github/workflows/onp-sdd-evidence.yml`, `.spec/features/backup-restore/spec.md`,
  `docs/database-restore.md`
- Evidência pendente até os gates do branch e do PR executarem no SHA final.

## Suposições

- A promoção atômica e o receipt são a fronteira desta entrega; rollback automático
  posterior e migration hardening permanecem cards seguintes.
