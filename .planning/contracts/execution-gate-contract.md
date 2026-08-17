# Execution Gate Contract — GOV-003

## Preflight obrigatório

O runner deve capturar `card_id`, repository, branch, worktree, base SHA, tree SHA, dirty state, scope, non-goals, allowed files/commands, author, reviewer, policy revision, schema revision e rollback antes de executar qualquer comando.

## Deny-before-write

- branch `main`/`master`, worktree inexistente ou dirty state não declarado: `BLOCKED`;
- path fora da allowlist, comando fora da policy ou card diferente do selecionado: `BLOCKED`;
- secret em env/log/artifact/fixture, scope drift ou migration/API não comprovada: `BLOCKED`;
- reviewer igual ao author: `BLOCKED`;
- base/tree/policy/schema divergente da evidência: `NO_PROOF` até reexecução.

## Evidência e lifecycle

Cada comando possui exit code e digest de output; cada artifact possui path e SHA-256; o review aponta para o mesmo SHA/tree/policy. Rebase, CI tardio, retry sem idempotency, cancelamento ou crash invalida o manifest anterior e exige estado terminal explícito.

## Integração do W0 gate

Um `PASS` de W0 exige cinco reports distintos (`ARCH-001`, `ARCH-002`, `GOV-001`, `GOV-002`, `GOV-003`) com status `PASS` e identidade igual de SHA, tree, policy e schema. Report ausente, duplicado, stale ou com status não aprovado retorna `NO_PROOF`/`BLOCKED`; `reason` e um objeto `evidence` isolados nunca são suficientes.

## Estados

O gate retorna somente `PASS`, `FAIL`, `BLOCKED` ou `NO_PROOF`, sempre com motivo e identidade. Um prompt ou comentário de agente nunca é aprovação.
