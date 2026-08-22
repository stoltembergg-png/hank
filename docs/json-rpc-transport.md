# JSON-RPC Transport

Transporte das worker messages (PR-112) via **JSON-RPC 2.0 com framing
`Content-Length`** (estilo LSP) sobre stdio. Implementações espelhadas:
`crates/agent-protocol/src/json_rpc.rs` (Rust) e
`python/runtime/transport.py` (Python, stdlib apenas). O transporte substitui
o NDJSON ad hoc do PR-113.

## Framing

```
Content-Length: <bytes>\r\n\r\n<compact JSON payload>
```

- Payload compacto (sem espaços), UTF-8, ≤64 KiB (`MAX_PAYLOAD_BYTES`)
- Frame completo ≤128 KiB (`MAX_FRAME_BYTES`)
- O decoder é incremental: frames fragmentados ou colados decodificam
  exatamente a mesma mensagem (testes golden cobrem byte-stability)

## Mensagens

```json
{"jsonrpc":"2.0","id":1,"method":"handshake","params":{...}}      // request
{"jsonrpc":"2.0","id":1,"result":{"kind":"handshake_accepted"...}} // response
{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"..."}}   // error
{"jsonrpc":"2.0","method":"cancel","params":{...}}                 // notification
```

Métodos = kinds do worker protocol (allowlist): `handshake`, `request`,
`cancel`, `health`, `error`, `shutdown`. `params`/`result` carregam os
campos da mensagem do protocolo (incluindo `schema_version`); responses
correlacionam o id exato do request. Notifications não têm resposta.

## Códigos de erro

| Código | Significado |
|---:|---|
| -32700 | parse error (JSON/schema/protocol version inválidos) |
| -32600 | invalid request (estrutura/id/params inválidos) |
| -32601 | method not found (método fora da allowlist) |
| -32602 | invalid params |
| -32603 | internal error |
| -32010 | frame excedente (oversize) |
| -32011 | request id duplicado (janela anti-replay) |
| -32012 | backpressure (capacidade de pendentes) |
| -32013 | request expirado |

Mensagens de erro são fixas, bounded (≤256 chars) e nunca ecoam payload.

## Correlação e limites

- Ids numéricos ≥0 registrados com deadline; duplicado em voo e capacidade
  (256 pendentes) falham fechado
- Conclusão/cancelamento têm estados definidos: `Completed`, `UnknownId`,
  `Expired`
- O worker mantém janela bounded de 256 ids recentes (anti-replay);
  reuso imediato → erro -32011
- Disconnect descarta frames parciais com estado definido

## Compatibilidade e rollback

Framing e códigos são contratuais e cobertos por golden tests nos dois
lados. Downgrade para o NDJSON do PR-113 não é suportido in-place; rollback
= reverter para o commit anterior ao transporte (o worker e o codec vivem
em commits isolados). Nova versão de transporte exige novo número de schema
e atualização espelhada dos dois codecs.
