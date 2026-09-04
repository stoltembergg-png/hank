# Tasks: database backups

> feature: database-backups

## T-1600 — Implementar snapshot SQLite, manifesto e retenção [em-andamento]

- Refs: US-1600, AC-1601, AC-1602, AC-1603, AC-1604, AC-1605
- Arquivos: crates/agent-runtime/src/backup.rs, crates/agent-runtime/src/lib.rs, crates/agent-runtime/tests/database_backup_contract.rs, docs/database-backups.md
- Evidência executada: teste contratual focado 8/8, suíte `agent-runtime` 46 unitários
  mais integrações, fmt, Clippy da biblioteca, arquitetura, actionlint e ONP 5/5.
- Limite conhecido: Clippy `--all-targets` local continua bloqueado por lint pré-existente
  em `crates/test-support/src/benchmark_comparison.rs`; a verificação oficial usa o
  toolchain pinned da CI.

## Suposições

- O adapter usa somente SQLite file-backed; bancos em memória falham fechado.
- Falhas de interrupção são modeladas por temporários/limite de tamanho; nenhum teste
  local simula perda de energia real.
