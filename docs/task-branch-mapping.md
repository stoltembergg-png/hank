# Task-to-branch mapping

## Boundary

`agent-core::task_mapping` define o contrato puro para vincular uma task a
projeto, repositório, worktree, branch, execução do agente, policy revision,
correlation ID e eventual pull request. O registry é bounded, determinístico e
não executa Git, filesystem, processos, rede ou capabilities.

`agent-runtime::TaskWorkspaceMappingRepository` é o adapter SQLite. Ele apenas
persiste os metadados do contrato e usa compare-and-set pela coluna `revision`.
A migração `0021_task_workspace_mappings.sql` mantém as chaves project-scoped:

- `(project_id, task_id)` identifica uma task;
- `(project_id, worktree_id)` impede dois owners do mesmo worktree;
- `(project_id, repository_id, branch)` impede branch duplicada no repositório;
- `(project_id, agent_run_id)` impede atribuição ambígua da execução.

A existência do projeto é verificada antes da inserção e a foreign key aplica
remoção em cascata no escopo do projeto. Nenhuma capability de Git é inferida
a partir do mapping.

## Lifecycle

```text
active ──detach──> detached ──resume──> active
   │                    │                 │
   └──reconcile────> reconcile_required  └──rebind──> active

active | detached | reconcile_required ──release──> released
```

- `detach`, `resume`, `reconcile`, `rebind` e `release` exigem a `revision`
esperada;
- `rebind` exige autorização explícita com a mesma `policy_revision` e razão
bounded;
- `reconcile` preserva somente a observação de repositório/worktree/branch e
marca divergência como `reconcile_required`;
- mapping `released` é terminal;
- restart reabre o banco e recupera o mesmo mapping, lifecycle, revision,
correlation ID e campos de reconciliação.

A reconciliação não resolve conflitos automaticamente. Um mapping divergente
fica bloqueado até rebind explícito e autorizado.

## Segurança e bounds

Todos os identificadores textuais, branch names, revisões, razões e IDs de PR
são bounded e rejeitam controle/traversal. IDs principais usam os tipos
`ProjectId`, `TaskId`, `RunId` e `TraceId` do protocolo. Erros do adapter são
tipados e não carregam payload bruto.

O mapping é evidência de identidade, não autorização. A validação de branch e
a decisão de mutação continuam pertencendo à policy de `security-core`; a
execução Git continua pertencendo ao adapter de `tool-core`.

## Rollback

A migração é aditiva e pode ser revertida operacionalmente somente por um
procedimento de backup/restore aprovado; o código não executa downgrade
silencioso. Em caso de falha de CAS, o chamador deve recarregar o mapping e
reconciliar, sem sobrescrever a revisão concorrente.

## Verificação

```bash
cargo test --package agent-core --test task_mapping_contract --locked
cargo test --package agent-runtime --test task_mapping_repository_contract --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
HANK_SKIP_TAURI=1 node tools/run-feature-tests.mjs task-branch-mapping
HANK_SKIP_TAURI=1 CI=1 node tools/ci/run-onp-spec.mjs verify task-branch-mapping
```

O artifact de `verify` é transitório e deve ser removido antes do commit,
conforme a política de evidência do repositório.
