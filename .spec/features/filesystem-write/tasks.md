# Tasks: Filesystem write

> feature: filesystem-write

## T-632 — Implementar write atômico e rollback bounded [concluida]

- Refs: US-607, AC-640, AC-641, AC-642
- Arquivos: crates/tool-core/src/filesystem_write.rs, crates/tool-core/src/lib.rs, crates/tool-core/tests/filesystem_write_contract.rs
- Notas: temp+rename no mesmo parent, snapshot em memória, rollback e dedupe por operation key.

## T-633 — Documentar write e rollback [concluida]

- Refs: US-607, AC-640, AC-641
- Arquivos: docs/filesystem-write.md
- Notas: permission gate, roots, atomicidade, snapshots e limites.
