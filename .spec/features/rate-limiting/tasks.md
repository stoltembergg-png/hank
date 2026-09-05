# Tasks: rate limiting

> feature: rate-limiting

## T-2570 — Policy portátil de token bucket [em-andamento]

- Refs: US-2570, AC-2571, AC-2572, AC-2573, AC-2574, AC-2575, AC-2576
- Arquivos: `crates/security-core/src/rate_limit.rs`,
  `crates/security-core/src/lib.rs`
- Entrega: chaves de escopo bounded; token bucket monotônico; retries
  idempotentes; bucket de recovery finito; snapshot/restore por revisão;
  métricas redigidas e erros fail-closed.
- Limites: nenhum backend de persistência, serviço distribuído ou quota de
  CPU/memória/disk.

## T-2571 — Aplicar limites em scheduler e ingresso remoto [em-andamento]

- Refs: US-2570, AC-2577, AC-2578
- Arquivos: `crates/remote-core`, `crates/agent-runtime`, `Cargo.lock`
- Entrega: bootstrap autenticado usa chave node/project depois do binding;
  scheduler limita trigger antes do claim; denial é explícita e auditável.
- Limites: nenhum listener/socket novo, serviço distribuído, auto-tuning ou
  execução de workflow/provider.

## T-2572 — Contratos, documentação e gate ONP [em-andamento]

- Refs: US-2570, AC-2571, AC-2572, AC-2573, AC-2574, AC-2575, AC-2576,
  AC-2577, AC-2578
- Arquivos: `crates/security-core/tests/rate_limiting_contract.rs`,
  `crates/remote-core/tests/authenticated_daemon_contract.rs`,
  `crates/agent-runtime/tests/scheduler_worker_contract.rs`,
  `docs/rate-limiting.md`, `.github/workflows/onp-sdd-evidence.yml`
- Evidência exigida: burst/refill, isolamento, replay, recovery, clock/snapshot,
  redaction, ingress remoto e trigger sem claim adicional.
- Status só pode mudar para `[concluída]` após testes locais, ONP e required
  checks passarem no SHA final da PR.
