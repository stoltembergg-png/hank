# Confirmation Bridge (Tauri/UI)

A ponte de confirmação conecta o `ConfirmationApplicationService` ao shell
desktop e à UI. É a única fronteira onde comandos de produto estão
registrados hoje, limitados ao ciclo de aprovação.

## Comandos

- `submit_confirmation_request` registra um `ApprovalRequest` e emite o
  evento `hank://confirmation` com payload `request_submitted`;
- `approve_confirmation_request` emite um `ApprovalGrant` para o actor
  apresentado (`input` com `request_id`/`actor_id`/`now_ms`);
- `revoke_confirmation_request` invalida o request ou o escopo `ask_once`.

Os comandos transportam somente o artefato serializável bounded. Erros são
mapeados para mensagens fixas e bounded (`ConfirmationBridgeError`); schema e
argumentos brutos nunca cruzam a ponte em comando, evento ou erro.

## Eventos

`ConfirmationEvent` carrega `schema_version` vigente (1), `event_id`,
`request_id`, `sequence` monotônica por processo e payload `request_submitted`
com o artefato completo. O guard `isConfirmationEvent` do frontend aceita
somente o schema vigente e requests com exatamente as chaves do artefato
bounded — qualquer campo extra (por exemplo payload bruto) invalida o evento.

## UI

`ConfirmationCard` renderiza um approval pendente com metadados bounded
(tool, versão, hash dos argumentos, efeito, policy, actor, expiração) e ações
acessíveis de aprovar/revogar. As ações vinculam o actor e o momento
apresentados (`nowMs`) e o card nunca renderiza o hash de schema ou payload
bruto.

## Limites deste incremento

Autenticação completa do actor, integração no loop de execução dos handlers
e persistência entre processos segem fora de escopo, conforme a spec de
confirmation policies.
