# Tasks: Git commit

> feature: git-commit
> status: implementada

## T-651 — Implementar Git commit explícito e autorizado [concluida]

- Refs: US-614, AC-659, AC-660, AC-661
- Arquivos: crates/tool-core/src/git_commit.rs, crates/tool-core/src/lib.rs, crates/tool-core/tests/git_commit_contract.rs, crates/tool-core/Cargo.toml
- Notas: preflight status, path validation contra git status, operation key dedupe, permission gating (Write + confirmação), author identity override, rollback documentado via git revert, sem push/force push/reset/amend/hooks arbitrários.

## T-652 — Documentar Git commit boundary [concluida]

- Refs: US-614, AC-659, AC-660, AC-661
- Arquivos: .spec/features/git-commit/spec.md, .spec/features/git-commit/tasks.md
- Notas: escopo (commit apenas), fora de escopo (push, force push, reset, amend, hooks, assinatura), preflight, validação paths, atomicidade, evidência.