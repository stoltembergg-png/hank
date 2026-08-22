# Tasks: Python worker

> feature: python-worker

## T-672 — Implementar worker mínimo com lifecycle controlado [concluida]

- Refs: US-620, AC-683, AC-684, AC-685
- Arquivos: python/runtime/worker.py, python/runtime/__main__.py, python/runtime/__init__.py
- Notas: loop NDJSON stdin/stdout com estados espelhando `WorkerSession`; handshake validado (versão, worker_id, capabilities) responde `handshake_accepted`; mensagens pré-handshake ou versão inválida encerram com exit 1; argumentos fora da allowlist encerram com exit 2; linhas malformadas/kinds desconhecidos respondem erro bounded e mantêm o canal; shutdown responde ack e encerra com exit 0.

## T-673 — Cobrir integração de processo e não-execução [concluida]

- Refs: US-620, AC-683, AC-684, AC-685, AC-686, AC-687
- Arquivos: crates/agent-protocol/tests/worker_process_contract.rs, .spec/verification/python-worker.json
- Notas: harness de processo com transcript; lifecycle feliz, negações fail-closed (exit 1/2), canal resiliente, request responde `not_supported` sem ecoar payload e a resposta valida como `WorkerMessage`; contrato de fonte sem manifestos de dependência e sem env/exec; sessão in-process sem Python.

## T-674 — Registrar verificação e documentar sidecar [concluida]

- Refs: US-620, AC-683, AC-684, AC-685, AC-686, AC-687
- Arquivos: .github/workflows/onp-sdd-evidence.yml, docs/python-worker.md
- Notas: passo `Verify python worker` no workflow de evidência; documentação de entrypoint, framing, lifecycle, exit codes, isolamento e rollback do sidecar.
