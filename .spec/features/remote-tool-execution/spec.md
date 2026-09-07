# Spec: remote tool execution boundary

> feature: remote-tool-execution
> status: implementada

## História de usuário

### US-3001 — Encaminhar uma chamada de ferramenta para o node autenticado

Como runtime remoto, quero encaminhar uma chamada tipada somente para o node e projeto
vinculados à lease autenticada, para que uma falha de transporte não vire uma nova
execução cega nem atravesse a fronteira de credenciais.

#### AC-3007 — Tool permitido executa no node exato e é idempotente

- **Dado** uma lease autenticada, uma política allowlist e uma ferramenta bounded
- **Quando** o dispatcher recebe a chamada para o node autenticado
- **Então** o transport recebe a identidade exata, a chamada completa uma vez e uma repetição
  do mesmo `OperationKey` retorna a resposta cacheada sem novo dispatch; a mesma chave com
  fingerprint diferente é rejeitada como conflito.

#### AC-3008 — Node, projeto, capability e permission gate são fail-closed

- **Dado** uma chamada com node divergente, projeto divergente, capability não autorizada ou
  decisão de permissão `Deny`
- **Quando** o dispatcher valida a chamada
- **Então** rejeita antes do transport e não cria efeito remoto.

#### AC-3009 — Payload de request não carrega material sensível

- **Dado** request serializado acima do limite ou contendo credential/token/password/secret
- **Quando** o boundary de transporte é preparado
- **Então** rejeita a chamada antes do transport.

#### AC-3010 — Falha após dispatch produz resultado desconhecido sem retry

- **Dado** transport que perde a conexão, excede timeout ou retorna protocolo inválido
- **Quando** a chamada já foi registrada para dispatch
- **Então** o estado terminal é `Unknown`, o chamador recebe `UnknownOutcome` e uma nova
  tentativa do mesmo `OperationKey` também é rejeitada.

#### AC-3011 — Cancelamento antes do dispatch não produz efeito

- **Dado** token de cancelamento já cancelado
- **Quando** a chamada chega ao dispatcher
- **Então** retorna `Cancelled` sem invocar o transport.

#### AC-3012 — Response divergente ou sensível não é aceita como sucesso

- **Dado** resposta com operação/trace/tool divergente ou material sensível
- **Quando** retorna do transport
- **Então** a operação termina como `UnknownOutcome`, sem cachear a resposta.

#### AC-3013 — Lease revogada ou expirada não pode despachar

- **Dado** uma lease que foi revogada ou expirou
- **Quando** uma chamada tenta usar seu ID
- **Então** retorna erro de lease stale antes do transport.

#### AC-3014 — Cancelamento em voo é terminal e late result vira desconhecido

- **Dado** operação em voo cujo adapter confirma cancelamento
- **Quando** uma resposta tardia chega após o cancelamento
- **Então** não é aceita como conclusão; o estado converge para `Unknown` e não há retry cego.

## Segurança e fronteira de prova

- O dispatcher é transport-neutral e usa `RemoteToolTransport` injetado.
- O DTO remoto mínimo não carrega `ToolContext`, decisão de policy, orçamento, reservation
  handle ou metadata arbitrária; o request local é validado antes da projeção.
- O ledger usa fingerprint do DTO para impedir que uma mesma `OperationKey` reutilize resposta
  para payload diferente.
- `CredentialRef` cru, token, password e secret não entram no payload aceito.
- `UnknownOutcome` é uma decisão fail-closed; reconciliação com o node é necessária antes
  de qualquer repetição.
- Esta feature prova somente o contrato offline com fixture sintética. Não prova WebSocket,
  TLS, listener, daemon de produção, execução em outro processo ou integração operacional.
  A classificação de produção permanece `NO_PROOF` até existir um adapter/runtime real e
  seus testes de integração.

## Suposições

- ASM-2105: um adapter de transporte autenticado implementará `RemoteToolTransport` em
  card posterior, preservando a mesma política de cancelamento e identidade.

## Perguntas em aberto

Nenhuma.
