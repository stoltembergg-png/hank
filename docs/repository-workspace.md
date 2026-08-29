# Repository workspace manager

## Boundary

PR-204 estabelece ownership seguro para workspaces usados por agentes de
Desenvolvimento. A implementação inicial vive em `agent-core` e é deliberadamente
pura:

- registra `workspace_id`, `project_id`, `repository_id` e `canonical_root`;
- valida identificadores, tamanho, controle, raiz absoluta e segmentos `.`/`..`;
- rejeita a mesma raiz em mais de um workspace/project;
- mantém no máximo um lease ativo por workspace;
- emite epochs monotônicos e exige o token exato para release;
- não acessa filesystem, executa Git/shell, persiste dados ou manipula secrets.

`canonical_root` é uma entrada de confiança limitada: um adapter de infraestrutura
precisa resolver symlinks e confirmar a raiz física antes de chamar o domínio. O
módulo de domínio não finge ter feito essa operação e não transforma um caminho
fornecido pelo usuário em uma prova de canonicalização física.

## Lease lifecycle

```text
registered → leased(epoch N) → registered → leased(epoch N+1)
                         └─ conflito determinístico para outro holder
```

O token contém somente `workspace_id`, `holder_id` e `epoch`. Um token ausente,
reutilizado ou pertencente a outro workspace falha com
`DomainError::ConcurrencyConflict`; o holder ativo nunca é substituído
silenciosamente.

O manager é in-memory nesta etapa. Persistência, restart recovery, dirty/unsupported
snapshot e fencing entre processos pertencem aos adapters/runtime dos incrementos
seguintes. Status/diff continuam sendo responsabilidades read-only dos contratos
PR-106/PR-107; este módulo não abre uma ponte de mutação Git.

## Verificação

A prova executável está em
`crates/agent-core/tests/workspace_contract.rs` e cobre:

- registro com ownership e raiz preservados;
- root relativo, traversal, controle e oversized rejeitados sem mutação;
- conflito concorrente determinístico;
- release exato, token stale e epoch monotônico;
- duplicata e cross-project sem alterar o registro original.

Comandos locais:

```bash
cargo test --package agent-core --test workspace_contract --locked
cargo test --package agent-core --locked
cargo check --package agent-core --locked
cargo clippy --package agent-core --all-targets --locked -- -D warnings
cargo fmt --all -- --check
git diff --check
```

O runner da feature também deve ser usado antes do `onp-spec verify`:

```bash
HANK_SKIP_TAURI=1 node tools/run-feature-tests.mjs repository-workspace
```

Nenhum comando desta etapa declara que canonicalização OS, persistência ou
recovery de restart estão implementados.
