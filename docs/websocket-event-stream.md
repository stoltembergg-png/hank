# WebSocket Event Stream Contract

`remote-core::event_stream` define um canal de eventos bounded, redigido e
transport-neutral, vinculado a uma lease autenticada do daemon.

## Fluxo

1. O stream é criado com uma política bounded
   (`EventStreamPolicy`): max events, max payload por evento e janela de replay.
2. `bind` vincula o stream à lease ativa exata; lease stale, revogada ou
   desconhecida falha fechado.
3. `push` atribui a próxima sequência monotônica; payload acima do limite ou
   buffer cheio falham fechado.
4. `ack` avança o watermark acked e evita eventos já entregues.
5. `resume` permite replay apenas dentro da janela a partir da sequência acked;
   fora da janela falha fechado.
6. `close` encerra o stream apenas para a lease à qual foi vinculado.

## Limites de segurança

- Nenhum socket, WebSocket, TLS, listener público, OAuth ou dispatch de eventos
  é implementado aqui — o contrato é transport-neutral.
- Toda admissão revalida a lease contra o daemon no `now_ms` informado; lease
  expirada, revogada ou substituída falha fechado.
- O payload é bounded e redigido; nada de credencial, token ou conteúdo de
  página cru é retido ou transmitido.
- Reconnect é negado por padrão; resume só é aceito dentro da janela com lease
  válida.
