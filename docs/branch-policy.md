# Branch policy

## Boundary

`security-core` contém uma policy pura para decidir mutações de branch. Ela não
executa Git, acessa GitHub, consulta filesystem, carrega credentials ou altera
sua própria configuração.

A decisão é vinculada a:

- `project_id` e `repository_id` da policy;
- `task_id`, `owner_id` e `actor_id` da solicitação;
- branch derivada exatamente de `branch_prefix + task_id`;
- revisão explícita da policy;
- operação solicitada.

O actor precisa ser o owner declarado da task. Escopo incompatível, branch que
não corresponde à task ou revisão stale falham fechado.

## Matriz de mutações

| Operação | Branch de task não protegida | Branch protegida |
|---|---|---|
| `LocalCommit` | permitido | negado |
| `Push` | permitido | negado |
| `ForcePush` | negado | negado |
| `Merge` | negado | negado |

`ForcePush` e `Merge` não possuem fallback permissivo. A policy não representa
enforcement de regras remotas: a lista local de protected branches é apenas uma
entrada declarativa para esta decisão bounded.

## Lifecycle

```text
policy carregada → request identificado → validação bounded → decisão explícita
```

O resultado `Allowed` inclui a revisão e a operação autorizada. Erros de
validação, escopo, ownership, branch, revisão e operação são tipados. A policy
não expõe métodos de mutação; uma nova configuração exige uma nova instância e
uma revisão diferente carregada pelo owner apropriado.

## Fora de escopo

- criação ou checkout de branch;
- commit, push, force-push ou merge reais;
- GitHub live rulesets, criação de PR ou aprovação remota;
- Persistência e reconciliação de task-to-branch mapping são responsabilidades do repository bounded em `agent-runtime` (PR-207); a policy continua sem acesso a storage.
- credentials, secrets, release signing e UI.

## Verificação

```bash
cargo test --package security-core --test branch_policy_contract --locked
cargo test --package security-core --locked
cargo clippy --package security-core --all-targets --locked -- -D warnings
cargo fmt --all -- --check
git diff --check
```
