# Tasks: Worker protocol

> feature: worker-protocol

## T-668 — Definir mensagens e validação do contrato [concluida]

- Refs: US-619, AC-677, AC-681, AC-682
- Arquivos: crates/agent-protocol/src/worker.rs, crates/agent-protocol/src/lib.rs
- Notas: `WorkerMessage` tagged serde com schema version vigente, contexto (projeto/sessão/task/trace), capability e payload bounded; validação fail-closed de versão, identidade, bounds e coerência resultado/valor/erro.

## T-669 — Implementar sessão com ciclo de vida e correlação [concluida]

- Refs: US-619, AC-678, AC-679, AC-680
- Arquivos: crates/agent-protocol/src/worker.rs
- Notas: `WorkerSession` valida a ordem handshake→ready→shutdown com estados tipados; correlaciona responses/cancels por id pendente; preserva o request em `ContextMismatch`; aplica backpressure bounded.

## T-670 — Cobrir contrato e evidência SDD [concluida]

- Refs: US-619, AC-677, AC-678, AC-679, AC-680, AC-681, AC-682
- Arquivos: crates/agent-protocol/tests/worker_protocol_contract.rs, .spec/verification/worker-protocol.json
- Notas: fixtures determinísticas cobrem happy path com serialização estável, ordenação fail-closed, correlação/replay, isolamento de contexto, bounds/versão/capacidade e erro/cancel sem instrução executável.

## T-671 — Registrar verificação e documentar protocolo [concluida]

- Refs: US-619, AC-677, AC-678, AC-679, AC-680, AC-681, AC-682
- Arquivos: .github/workflows/onp-sdd-evidence.yml, docs/worker-protocol.md
- Notas: passo `Verify worker protocol` no workflow de evidência; documentação de mensagens, estados, compatibilidade, threat boundary e rollback de versão.
