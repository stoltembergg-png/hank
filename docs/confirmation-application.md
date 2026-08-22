# Confirmation Application API

`ConfirmationApplicationService` é a primeira fronteira de aplicação do ciclo
de aprovação humana. Ele vive no `agent-runtime` junto dos demais serviços de
aplicação e delega as invariantes ao `ConfirmationLedger` do `tool-core`.

## Operações

- `submit` registra um `ApprovalRequest` e devolve o mesmo artefato bounded;
- `approve` emite um `ApprovalGrant` para o actor apresentado;
- `revoke` invalida o request ou o escopo `ask_once`;
- `authorize` executa a validação final antes do efeito sensível.

Requests carregam hashes de schema e argumentos; a API não recebe nem devolve
o payload bruto para a UI. Actor, projeto, tool, policy, trace, budget,
expiração e replay continuam vinculados pelo ledger. A ponte Tauri/UI,
autenticação completa do actor e eventos projetados ficam no próximo
incremento.
