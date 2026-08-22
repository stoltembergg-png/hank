# Worker Protocol

Contrato versionado e mínimo entre o Agent Runtime e um worker Python
opcional (`crates/agent-protocol/src/worker.rs`). O contrato é neutro: não
cria processo, não define transporte e não exige Python — o core compila,
testa e opera sem ele (D-006, AI-016).

## Mensagens

`WorkerMessage` (schema version 1, serialização determinística):

- `handshake` — abre a sessão com identidade do worker, versão do protocolo
  e capabilities declaradas (1..=32);
- `handshake_accepted` — completa o handshake ecoando a versão acordada;
- `request` — carrega `request_id`, `WorkerContext` (projeto, sessão,
  task opcional, trace), `capability` e payload bounded (≤64 KiB);
- `response` — correlacionado por `request_id` com o mesmo contexto;
  resultado terminal (`succeeded/rejected/failed/cancelled/timed_out/
  not_supported/blocked`) com coerção: sucesso não carrega erro, rejeição
  exige detalhe bounded, cancelamento/timeout não carregam valor;
- `cancel` — somente `request_id` e motivo bounded (`user/deadline/
  session_closed/shutdown`); nunca instrução executável;
- `health` / `health_report` — probe e status (`healthy/degraded/unhealthy`);
- `error` — código de protocolo + detalhe bounded (≤256 chars, sem controle);
- `shutdown` / `shutdown_ack` — encerramento explícito e confirmação.

## Estados

`WorkerSession` valida a conversa na ordem observada no canal:

```
AwaitingHandshake → Handshaking → Ready → ShuttingDown → Shutdown
```

Violações tipadas e fail-closed: `NotHandshaked`, `AlreadyHandshaked`,
`InvalidState`, `AfterShutdown`, `DuplicateRequest`, `UnknownRequest`,
`ContextMismatch`, `Backpressure`, `UnsupportedVersion`, `InvalidIdentity`,
`InvalidPayload`, `OversizedPayload`. Em `ContextMismatch` o request
permanece pendente — somente o response válido o consome.

## Compatibilidade e rollback

A v1 aceita somente a versão vigente do schema (`UnsupportedVersion`
caso contrário). Nova versão de schema exige novo número em
`WORKER_PROTOCOL_SCHEMA_VERSION` e tolerância explícita no validador;
rollback significa voltar a aceitar somente a versão anterior, sem
negociação implícita.

## Threat boundary

- O contrato é dados bounded: erros e cancelamentos nunca carregam
  instrução executável nem segredo; mensagens de erro são fixas e redigidas.
- Isolamento: responses devem devolver o contexto exato do request
  (projeto/sessão/trace); divergência falha fechadamente.
- Capacidade: no máximo 256 requests pendentes (`Backpressure` fail-closed).
- O worker não recebe capability por confiança transitiva do processo host
  (AI-017): capabilities são declaradas no handshake e decididas pelo
  Permission Engine no runtime.
