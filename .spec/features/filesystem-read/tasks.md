# Tasks: Filesystem read

> feature: filesystem-read

## T-630 — Implementar read-only filesystem tool [concluida]

- Refs: US-606, AC-635, AC-636, AC-637, AC-638, AC-639
- Arquivos: crates/tool-core/src/filesystem_read.rs, crates/tool-core/src/lib.rs, crates/tool-core/tests/filesystem_read_contract.rs
- Notas: roots canônicas, project/permission gate, traversal/absolute/symlink rejection, UTF-8 estrito, truncamento bounded e ausência de mutação.

## T-631 — Documentar limites e ameaça de filesystem read [concluida]

- Refs: US-606, AC-635, AC-636, AC-637
- Arquivos: docs/filesystem-read.md
- Notas: roots, canonicalização, symlink, UTF-8, limites e não-escopo de write/process.
