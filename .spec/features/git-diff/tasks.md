# Tasks: Git diff

> feature: git-diff

## T-644 — Implementar Git diff read-only bounded [concluida]

- Refs: US-613, AC-656, AC-657, AC-658
- Arquivos: crates/tool-core/src/git_diff.rs, crates/tool-core/src/lib.rs, crates/tool-core/tests/git_diff_contract.rs
- Notas: staged/unstaged/path, argv sem shell, redaction de secrets/control chars, truncamento e permission/project/path gates.

## T-645 — Documentar Git diff boundary [concluida]

- Refs: US-613, AC-656, AC-657
- Arquivos: docs/git-diff.md
- Notas: modos, limites, conteúdo não confiável e ausência de apply/mutação.
