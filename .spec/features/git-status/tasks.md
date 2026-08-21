# Tasks: Git status

> feature: git-status

## T-642 — Implementar Git status read-only [concluida]

- Refs: US-612, AC-654, AC-655
- Arquivos: crates/tool-core/src/git_status.rs, crates/tool-core/src/lib.rs, crates/tool-core/tests/git_status_contract.rs
- Notas: argv estruturado, GIT_OPTIONAL_LOCKS=0, root/project/permission gate, porcelain parser e entry bound.

## T-643 — Documentar Git status boundary [concluida]

- Refs: US-612, AC-654, AC-655
- Arquivos: docs/git-status.md
- Notas: read-only, parser, limites e ausência de hooks/mutação.
