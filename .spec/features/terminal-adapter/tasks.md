# Tasks: Terminal adapter

> feature: terminal-adapter

## T-638 — Implementar terminal adapter sobre process primitive [concluida]

- Refs: US-610, AC-649, AC-650
- Arquivos: crates/tool-core/src/terminal.rs, crates/tool-core/src/lib.rs, crates/tool-core/tests/terminal_contract.rs
- Notas: delegação única, round cap, operation key dedupe e preservação de erro.

## T-639 — Documentar terminal boundary [concluida]

- Refs: US-610, AC-649, AC-650
- Arquivos: docs/terminal-adapter.md
- Notas: terminal usa process primitive; PTY/shell ficam fora.
