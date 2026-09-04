# Tasks: secret migration

> feature: secret-migration

## T-1900 — Coordenador de migração privacy-preserving [em-andamento]

- Refs: US-1900, AC-1901, AC-1902, AC-1903, AC-1904, AC-1905, AC-1906, AC-1907,
  AC-1908
- Arquivos: `crates/secrets-core/src/migration.rs`,
  `crates/secrets-core/src/lib.rs`
- Entrega: ports de fonte, codec, staging, destino, clock e journal; preflight
  de escopo/política; estados `Started`/`Staged`/`DestinationWritten`/
  `Verified`/`Applied`/`Quarantined`; retry explícito e cutover após verificação.
- Limites: nenhum adapter real de OS keychain/Stronghold, formato legado de
  produção ou claim de migração executada em produção.

## T-1901 — Contratos negativos, documentação e gate ONP [em-andamento]

- Refs: US-1900, AC-1901, AC-1902, AC-1903, AC-1904, AC-1905, AC-1906, AC-1907,
  AC-1908
- Arquivos: `crates/secrets-core/tests/secret_migration_contract.rs`,
  `docs/secret-migration.md`, `.github/workflows/onp-sdd-evidence.yml`
- Evidência exigida: escopo cruzado, autorização expirada/sem consentimento,
  envelope bounded sem plaintext, falha de verificação, quarentena, retry sem
  releitura e retry idempotente.
- Status só pode mudar para `[concluída]` após testes locais, ONP e required
  checks passarem no SHA final da PR.
