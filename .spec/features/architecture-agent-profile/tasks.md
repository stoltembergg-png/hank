# Tasks: architecture agent profile

> feature: architecture-agent-profile

## T-1334 — Implementar architecture manifest e graph checks [concluida]

- Refs: US-1334, AC-1334
- Escopo: manifest tipado, layers, edges, allowlist, forbidden edges e cycle checks.
- Arquivos: `crates/agent-core/src/architecture_profile.rs`,
  `crates/agent-core/tests/architecture_agent_profile_contract.rs`.
- Não-escopo: editar arquitetura, executar source/commands ou ratificar ADR.
- Segurança: parser bounded, fail-closed e sem capability executável.

## T-1335 — Implementar architecture evidence e document impact [concluida]

- Refs: US-1335, AC-1335
- Escopo: graph revision, SHA/tree/policy, ADR/docs references, status e digests.
- Arquivos: `crates/agent-core/src/architecture_profile.rs`,
  `docs/architecture-agent-profile.md`.
- Não-escopo: publicar artifacts, alterar gate ou inferir PASS de evidence ausente.
- Segurança: stale/missing/skipped/no-run/malformed permanece NoProof/Blocked.

## T-1336 — Implementar advisory finding handoff [concluida]

- Refs: US-1336, AC-1336
- Escopo: findings bounded e handoff read-only com digest e authority negativa.
- Arquivos: `crates/agent-core/src/architecture_profile.rs`,
  `crates/agent-core/tests/architecture_agent_profile_contract.rs`.
- Não-escopo: refactor automático, ADR approval, graph-gate bypass ou merge.
- Segurança: texto hostil/instruction-like é rejeitado como dado, nunca executado.

- DoD: testes focalizados, feature runner, verify ONP, gates locais e docs passam;
  artefatos de verificação gerados não entram no commit.
- Status: concluído após testes focais, feature runner, verify ONP e gates locais; artefatos de verificação gerados não entram no commit.
