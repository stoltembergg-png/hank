# Tasks: Git worktree manager

> feature: git-worktree-manager

## T-1304 — Definir registry puro de worktrees [concluida]

- Refs: US-1302, AC-1306, AC-1307, AC-1308
- Arquivos: crates/agent-core/src/worktree.rs, crates/agent-core/src/lib.rs, crates/agent-core/tests/worktree_contract.rs
- Notas: registra task/workspace/owner/mode, valida containment lexical e mantém idempotência sem I/O.

## T-1305 — Implementar adapter Git bounded [concluida]

- Refs: US-1302, AC-1309, AC-1310
- Arquivos: crates/tool-core/src/git_worktree.rs, crates/tool-core/tests/git_worktree_contract.rs
- Notas: argv explícito para add/list/remove, output bounded e parser fail-closed de `git worktree list`.

## T-1306 — Cleanup recovery, documentação e auditoria [pendente]

- Refs: US-1302
- Arquivos futuros: documentação do lifecycle e integração de auditoria
- Notas: orphan recovery somente dentro da allowlist, owner authorization, testes de failure modes e prova ONP.
