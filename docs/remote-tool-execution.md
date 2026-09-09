# Remote tool execution boundary

## O que esta fatia entrega

`remote-core::tool_dispatch` liga uma `DaemonLease` autenticada a um request de
`tool-core` por meio de uma porta `RemoteToolTransport`. Antes de chamar a porta,
o dispatcher valida:

- lease ainda ativa, node exato e projeto exato;
- allowlist de nome, versão e capability;
- decisão do `PermissionEvaluator`;
- timeout, deadline e tamanho de request/response;
- DTO remoto mínimo, sem `ToolContext`, policy, orçamento, reservation handle ou metadata
  arbitrária;
- ausência de credential/token/password/secret em valores e campos sensíveis;
- `OperationKey` com fingerprint: replay idempotente somente para o mesmo request e conflito
  fail-closed para payload diferente.

O fingerprint canônico exclui o deadline calculado, então a repetição legítima da
mesma chamada pode ocorrer em outro instante sem virar conflito. Estados terminais
são retidos por 10 minutos (`REMOTE_OPERATION_RETENTION_MS`); depois disso a capacidade
é recuperada e a chave antiga não deve ser reutilizada sem reconciliação explícita.
Registros `InFlight` não expiram por essa rotina.

O dispatcher aplica `tokio::time::timeout` ao future do transport. Se o prazo vence
após o dispatch, o future local é descartado e o resultado é `UnknownOutcome`.

O ledger mantém estados `InFlight`, `Completed`, `Cancelled`, `Rejected` e
`Unknown`. Timeout, indisponibilidade, resposta inválida, cancelamento tardio ou
perda de transporte nunca são automaticamente repetidos: retornam
`UnknownOutcome` e exigem reconciliação explícita com o node.

## O que não está provado

A porta é deliberadamente injetada. A fixture de contrato é offline e não abre
socket, não inicia daemon, não executa shell, não resolve credential e não chama
provider. Portanto, esta feature pode obter `CONTRACT PASS`, mas continua
`PRODUCTION NO_PROOF` até existir um adapter autenticado de WebSocket/TLS, um
runtime remoto separado e testes de integração vinculados ao mesmo SHA/tree.

## Verificação local

```bash
CARGO_BUILD_JOBS=1 cargo test -p remote-core --test remote_tool_dispatch_contract --locked --offline
```

O teste cobre execução no node correto, negação de node/projeto/permissão, redaction,
idempotência com deadline variável, conflito de fingerprint, retenção do ledger,
timeout real, unknown outcome, cancelamento antes/em voo, resposta divergente e
lease revogada/expirada. Ele não substitui a prova de rede ou de produção.
