# Git worktree manager

## Boundary

O `git-worktree-manager` separa a intenção de worktree, em `agent-core`, da
execução de Git, em `tool-core`.

- `WorktreeRegistry` mantém requests bounded por task, workspace, projeto e owner.
- O registry não acessa filesystem, não executa shell/Git e não canonicaliza
  fisicamente paths.
- `GitWorktreeAdapter` recebe uma raiz de repositório autorizada e usa argv
  estruturado para `git worktree add`, `git worktree list --porcelain` e
  `git worktree remove` sem `--force`.
- A canonicalização física e a autorização da raiz pertencem ao adapter externo
  responsável por fornecer a entrada de confiança limitada.

## Recovery dry-run

`WorktreeRegistry::recovery_plan` transforma um snapshot já estruturado de paths
observados em ações determinísticas. O método não remove nada e não altera o
registry.

Um path só recebe `RemoveRegistered` quando as duas condições são verdadeiras:

1. corresponde exatamente a um path registrado;
2. o registro pertence ao `project_id` e ao `owner_id` fornecidos ao plano.

Paths desconhecidos, de outro projeto ou de outro owner recebem
`PreserveUnknown`. Eles devem permanecer intocados até existir registro e
autorização explícitos. Essa regra evita que um snapshot stale ou foreign seja
interpretado como autorização destrutiva.

O plano também rejeita IDs inválidos, paths relativos, traversal, caracteres de
controle, paths oversized e quantidade oversized de observações antes de
produzir qualquer ação. A ordenação é estável: ações de preservação vêm antes
de ações de remoção, e cada grupo é ordenado pelo path.

## Lifecycle suportado neste slice

```text
request validado → registro bounded → adapter add/list/remove
                                  └→ recovery_plan dry-run
```

Persistência, restart recovery automático, detecção completa de dirty state,
canonicalização física, orphan recovery destrutivo, checkout, commit, push e
política de branches permanecem fora deste slice.

## Verificação

```bash
cargo test --package agent-core --test worktree_contract --locked
cargo test --package agent-core --locked
cargo clippy --package agent-core --all-targets --locked -- -D warnings
HANK_SKIP_TAURI=1 node tools/run-feature-tests.mjs git-worktree-manager
HANK_SKIP_TAURI=1 CI=1 node tools/ci/run-onp-spec.mjs verify git-worktree-manager
```

O artifact gerado por `onp-spec verify` é evidência local transitória e não deve
ser publicado quando a política do repositório exigir sua remoção.
