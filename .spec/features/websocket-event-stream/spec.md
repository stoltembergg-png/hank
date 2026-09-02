# Spec: websocket event stream

> feature: websocket-event-stream
> status: em-implementacao

### US-1450 — Stream de eventos autenticado e bounded

Como runtime remoto, quero um canal de eventos vinculado a uma lease autenticada,
com ordenação, ack, buffer bounded, reconnect autorizado e redação determinística,
para que nenhum evento cruze o boundary sem identidade, fora de ordem, além do
buffer ou após revogação.

#### AC-1461 — Stream rejeita eventos sem lease autenticada válida

- **Dado** stream associado a uma lease de daemon autenticada
- **Quando** uma mensagem tenta cruzar sem lease ativa, com lease expirada ou com
  lease revogada
- **Então** a operação falha fechado com erro tipado e não entrega o evento.

#### AC-1462 — Sequência de eventos rejeita duplicados e gaps não declarados

- **Dado** stream com contador de sequência monotônico e janela de replay bounded
- **Quando** um evento com sequência ≤ última, um gap sem ack correspondente ou
  um replay fora da janela é apresentado
- **Então** o stream rejeita com erro tipado e mantém o estado consistente.

#### AC-1463 — Buffer bounded aplica backpressure e descarta política

- **Dado** fila de eventos com limite máximo de itens e bytes
- **Quando** a fila atinge o limite ou um payload excede o tamanho máximo
- **Então** novos eventos são negados ou o evento é marcado dropped conforme a
  política; o stream nunca excede o limite.

#### AC-1464 — Reconnect só é autorizado com lease válida e resume de sequência

- **Dado** stream desconectado e peer tentando reconectar
- **Quando** a lease continua válida e a sequência de resume está dentro da janela
- **Então** o reconnect é aceito e o replay começa da sequência ack'd;
  caso contrário, reconnect é negado com erro tipado.

#### AC-1465 — Eventos são redigidos e nunca expõem material de credencial

- **Dado** evento com payload bounded
- **Quando** o evento é registrado, transmitido ou logado
- **Então** o payload não contém credencial, token ou conteúdo de página cru;
  campos sensíveis são redigidos de forma determinística.

## Segurança

- A identidade do stream é vinculada à lease exata (peer/node/project/revision);
  stale cleanup não pode fechar uma sessão substituta.
- Reconnect negado por padrão; somente lease válida + resume dentro da janela.
- Sem socket, listener, HTTP/WebSocket, OAuth callback ou dispatch remoto nesta
  fatia — o contrato é transport-neutral.

## Suposições

- ASM-1460: adapters concretos de WebSocket, TLS, bind e dispatch de eventos
  pertencem a cards posteriores, mantendo este core sem dependência de rede.

## Perguntas em aberto

Nenhuma.
