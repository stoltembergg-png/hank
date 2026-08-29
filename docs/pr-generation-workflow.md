# PR Generation Workflow

`pr-generation-workflow` é um contrato puro em `agent-core` para transformar um
handoff de coding em um plano declarativo de PR draft. O módulo não chama GitHub
nem executa qualquer efeito externo.

## Contrato

O handoff exige mapping `Active` e identidade exata de projeto, task, repository,
worktree, branch, `head_sha`, `tree_sha` e `policy_revision`. Também exige
idempotency key, objetivo, escopo, não-escopo, testes, acceptance criteria,
riscos, rollback, documentação, paths relativos e quatro evidências bounded:
tests, security, scope e evidence.

SHA, paths, quantidade de itens, tamanho do corpo e metadata são bounded. Campos
vazios, controls, traversal, secret-like ou instruction-like são rejeitados
fail-closed. Evidência `Failed`, `Skipped` ou `NoRun` não gera plano.

## Plano draft-only

- Sem draft existente: `CreateDraft`.
- Com `existing_draft_id`: `UpdateDraft` idempotente.
- O plano carrega identidade, SHA/tree, idempotency key e fingerprint estável.
- `can_publish()` e `can_merge()` são sempre `false`.

O adapter GitHub, autenticação, persistência e publicação ficam fora do domínio e
precisam de seus próprios contratos de permissão, stale protection e rollback.
Metadata do handoff é dado não confiável; nunca é shell, prompt de autoridade,
approval, capability ou instrução de merge.

## Rastreadibilidade e rollback

A feature é rastreada por `US-1337..US-1339`, `AC-1337..AC-1339` e
`T-1337..T-1339`. O rollback remove o módulo, contrato, documentação, SDD/tasks e
etapa ONP, sem tocar branches, GitHub, credentials ou releases.
