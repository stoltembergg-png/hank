# Tasks: rate limiting

> feature: rate-limiting

## T-2000 — Implementar policy token-bucket bounded [concluida]

- Refs: US-2000, AC-2001, AC-2002, AC-2003
- Arquivos: `crates/security-core/src/rate_limit.rs`, `crates/security-core/src/lib.rs`,
  `crates/security-core/tests/rate_limit_contract.rs`
- Evidência local: `cargo test -p security-core --test rate_limit_contract --locked --offline` — 5/5.
- A policy valida burst/window/revisão, relógio monotônico, identidade por projeto, retry
  idempotente, recovery/métricas bounded e capacidade máxima de estado.

## T-2001 — Aplicar limite ao ingresso remoto autenticado [concluida]

- Refs: AC-2002, AC-2003, AC-2004
- Arquivos: `crates/remote-core/Cargo.toml`, `crates/remote-core/src/lib.rs`,
  `crates/remote-core/tests/authenticated_daemon_contract.rs`, `docs/authenticated-remote-daemon.md`
- Evidência local: `cargo test -p remote-core --test authenticated_daemon_contract --locked --offline` — 5/5.
- Excesso autenticado retorna `RateLimited`, registra auditoria redigida e não cria lease.

## T-2002 — Aplicar gate ao dispatch de agente [concluida]

- Refs: AC-2002, AC-2003, AC-2005
- Arquivos: `crates/agent-runtime/Cargo.toml`, `crates/agent-runtime/src/agent_scheduler.rs`,
  `crates/agent-runtime/tests/agent_scheduler_contract.rs`, `docs/agent-scheduler-integration.md`
- Evidência local: `cargo test -p agent-runtime --test agent_scheduler_contract --locked --offline` — 4/4.
- O gate não produz dispatch quando negado e mantém projetos distintos separados.

## T-2003 — Registrar contrato SDD e runbook operacional [concluida]

- Refs: AC-2001, AC-2002, AC-2003, AC-2004, AC-2005
- Arquivos: `.github/workflows/onp-sdd-evidence.yml`, `.spec/features/rate-limiting/spec.md`,
  `.spec/features/rate-limiting/tasks.md`, `docs/rate-limiting.md`
- Evidência local: `verify rate-limiting` — 5/5; feature runner — 3 comandos/14 testes;
  actionlint, arquitetura e contratos de workflow passaram.
- Audit global: `NO_PROOF` baseline do repositório (`841` erros/`227` avisos); a feature
  não introduziu erro, somente 5 avisos de granularidade `PROVA_FRACA`.
- Evidência remota fica pendente até o SHA final da PR.

## Suposições

- A policy e seus adapters permanecem em memória nesta fatia; persistência/rede distribuída
  não é inferida por testes de contrato.
