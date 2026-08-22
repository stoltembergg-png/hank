# Tasks: JSON-RPC transport

> feature: json-rpc-transport

## T-675 — Implementar codec JSON-RPC em Rust [concluida]

- Refs: US-621, AC-688, AC-689, AC-690, AC-693
- Arquivos: crates/agent-protocol/src/json_rpc.rs, crates/agent-protocol/src/lib.rs
- Notas: frames `Content-Length` bounded (payload ≤64 KiB, frame ≤128 KiB); decoder incremental com estados Idle/Partial/Disconnected; mensagens request/response/error/notification untagged-serde com allowlist de métodos; correlação por id com deadline e limite 256; códigos de erro documentados (-32700/-32600/-32601/-32602/-32603/-32010/-32011/-32012/-32013) com mensagens fixas redigidas.

## T-676 — Implementar transporte espelhado no worker Python [concluida]

- Refs: US-621, AC-691, AC-692
- Arquivos: python/runtime/transport.py, python/runtime/worker.py
- Notas: codec espelhado (framing, bounds, códigos); worker.py migra de NDJSON para dispatch JSON-RPC com janela bounded anti-replay de ids (256); handshake inválido responde erro bounded e encerra exit 1; methods desconhecidos → -32601; frames malformados rejeitados sem eco e canal segue utilizável.

## T-677 — Cobrir contrato e processo nos dois lados [concluida]

- Refs: US-621, AC-688, AC-689, AC-690, AC-691, AC-692, AC-693
- Arquivos: crates/agent-protocol/tests/json_rpc_contract.rs, crates/agent-protocol/tests/worker_process_contract.rs, .spec/verification/json-rpc-transport.json, .spec/verification/python-worker.json
- Notas: golden bytes estáveis; fragmentação/coalescing; fuzz bounded sem panic; disconnect com estado definido; correlação (duplicado/capacidade/expiração); end-to-end com worker real (handshake/health/request/shutdown correlatos, erro -32011 em id duplicado, canal sobrevive a frame malformado); resultados continuam desserializando como `WorkerMessage`.

## T-678 — Registrar verificação e documentar framing [concluida]

- Refs: US-621, AC-688, AC-689, AC-690, AC-691, AC-692, AC-693
- Arquivos: .spec/features/python-worker/spec.md, .github/workflows/onp-sdd-evidence.yml, docs/json-rpc-transport.md, docs/python-worker.md
- Notas: contexto da spec python-worker atualizado para o framing JSON-RPC; passo `Verify json-rpc transport` no workflow; documentação de framing, limits, códigos de erro e downgrade/rollback.
