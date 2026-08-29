# Tasks: security agent profile

> feature: security-agent-profile

## T-1331 — Implementar security profile e threat evidence [concluida]

- Refs: US-1331, US-1332, US-1333, AC-1331, AC-1332, AC-1333
- Escopo: manifest de ameaças, evidence bound e handoff advisory em
  `security-core`.
- Arquivos: `crates/security-core/src/security_profile.rs`,
  `crates/security-core/tests/security_agent_profile_contract.rs`,
  `docs/security-agent-profile.md`.
- Não-escopo: exploração real, secrets, execução externa, gate mutation,
  approval, merge ou release.
- Testes: contrato positivo/negativo de allowlist, scope, stale SHA/tree/policy,
  missing fixture, malformed evidence, hipótese e artifact hostil.
- Segurança: fail-closed, sem raw prompt/log/payload, sem authority executável.
- Docs/rollback: `docs/security-agent-profile.md`; remoção isolada do contrato.
- DoD: verify ONP e gates locais passam; artefatos gerados não entram no commit.

- Status: em andamento; dependências PR-209 e PR-210 integradas.
