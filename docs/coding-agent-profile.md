# Coding agent profile

## Boundary

`agent-core::coding_profile` é um contrato puro e provider-neutral para uma
execução de coding associada a um `TaskWorkspaceMapping` ativo. Ele valida
somente schema, policy revision, identidade, ferramenta, path relativo, budget,
autonomia, cancelamento e handoff. Não executa Git, filesystem, processo, rede,
provider, publicação, merge ou qualquer mutação externa.

O mapping de PR-207 continua sendo a fonte de identidade: projeto, task,
repositório, worktree e branch precisam coincidir exatamente. Mapping detached,
reconcile-required ou released não recebe autorização.

## Profile e autorização

`CodingAgentProfile::default()` permite somente leitura/escrita de paths
relativos, aplicação de patch e execução do conjunto de testes. Comandos
arbitrários, rede, publicação e merge são proibidos na allowlist e também
rejeitados quando aparecem como intenção da requisição.

A autonomia é deny-by-default para rede, publicação e merge. Tentativas são
limitadas a no máximo três; todos os limites de tokens, invocações e wall time
são validados antes de produzir um `CodingPermit`. O permit contém apenas
identidade e policy revision e nunca contém uma capability de publicação ou
merge.

Paths absolutos, traversal (`..`), separadores Windows, controles, prefixo de
unidade e paths vazios são rejeitados. Texto recebido em request ou handoff é
dado não confiável; não pode alterar a policy.

## Handoff

`CodingAgentHandoff` é uma proposta, não aprovação. Carrega apenas:

- identity exacta do mapping e profile revision;
- lista bounded de paths alterados;
- digest hexadecimal do patch e do relatório;
- resultado dos required checks.

A validação exige mapping ativo, status `proposed`, identidade atual, paths
relativos, digests de 64 caracteres hexadecimais minúsculos e todos os checks
required como `passed`. `failed`, `skipped`, `no_run`, missing, duplicados,
stale ou claims de autoridade são rejeitados. O tipo não expõe métodos que
aprovem ou façam merge.

## Verificação

```bash
cargo test --package agent-core --test coding_agent_profile_contract --locked
HANK_SKIP_TAURI=1 node tools/run-feature-tests.mjs coding-agent-profile
HANK_SKIP_TAURI=1 CI=1 node tools/ci/run-onp-spec.mjs verify coding-agent-profile
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

O JSON produzido pelo verify é transitório e não deve ser commitado. O
workflow `ONP SDD verify and audit` executa a verificação da feature junto com
as verificações existentes, sem substituir a cobertura anterior.
