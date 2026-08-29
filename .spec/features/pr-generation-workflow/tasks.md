# Tasks: PR generation workflow

> feature: pr-generation-workflow

## T-1337 — Draft handoff bounded [concluida]

- Refs: US-1337, AC-1337
- Arquivos: crates/agent-core/src/pr_generation_workflow.rs, crates/agent-core/tests/pr_generation_workflow_contract.rs, .spec/features/pr-generation-workflow/spec.md, .spec/features/pr-generation-workflow/tasks.md
- Escopo: handoff, identity, required fields, bounds e mapping validation.
- Não-escopo: GitHub API, filesystem, Git ou publicação.

## T-1338 — Draft-only idempotency [concluida]

- Refs: US-1338, AC-1338
- Arquivos: crates/agent-core/src/pr_generation_workflow.rs, crates/agent-core/tests/pr_generation_workflow_contract.rs, docs/pr-generation-workflow.md
- Escopo: fingerprint, idempotency key, create/update plan e autoridade negativa.
- Não-escopo: merge, approval, release ou credentials.

## T-1339 — Hostile metadata [concluida]

- Refs: US-1339, AC-1339
- Arquivos: crates/agent-core/src/pr_generation_workflow.rs, crates/agent-core/tests/pr_generation_workflow_contract.rs, .github/workflows/onp-sdd-evidence.yml, test/aggregate-runner-native-boundary.js
- Escopo: rejeição bounded de traversal, controls, secrets e instruction-like text.
- Não-escopo: interpretação/execução de conteúdo externo.

- Status: concluído após testes focais, feature runner, verify ONP e gates locais; artefatos gerados não entram no commit.
