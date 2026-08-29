# Tasks: repository workspace manager

> feature: repository-workspace

## T-1301 — Definir contrato bounded de workspace [concluida]

- Refs: US-1301, AC-1301, AC-1302, AC-1305
- Arquivos: crates/agent-core/src/workspace.rs, crates/agent-core/src/lib.rs, crates/agent-core/tests/workspace_contract.rs
- Notas: raiz recebida canonicalizada, validação lexical sem filesystem, ownership project/repository e rejeição de duplicatas/cross-project.

## T-1302 — Implementar lease exclusivo em memória [concluida]

- Refs: US-1301, AC-1303, AC-1304
- Arquivos: crates/agent-core/src/workspace.rs, crates/agent-core/tests/workspace_contract.rs
- Notas: epoch monotônico, token exato, conflito determinístico e release sem substituição silenciosa.

## T-1303 — Documentar boundary e auditar feature [concluida]

- Refs: US-1301, AC-1301, AC-1302, AC-1303, AC-1304, AC-1305
- Arquivos: docs/repository-workspace.md, .spec/features/repository-workspace/spec.md, .github/workflows/onp-sdd-evidence.yml
- Notas: separar prova de domínio da canonicalização/integração OS, status/diff/Git e recovery futuro.
