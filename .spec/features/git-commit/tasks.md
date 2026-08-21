# Tasks: Git commit

> feature: git-commit
> status: implementada

## Tasks

- [x] PR-108: GitCommitTool em `crates/tool-core/src/git_commit.rs`
- [x] Contract tests em `crates/tool-core/tests/git_commit_contract.rs`
- [x] Export em `crates/tool-core/src/lib.rs`
- [x] Dev-dependency `which` em `crates/tool-core/Cargo.toml`

## Verificação

- `cargo fmt --check` ✓
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` ✓
- `cargo test --workspace` ✓
- ONP SDD verify e audit ✓ (após registrar AC-659/660/661)