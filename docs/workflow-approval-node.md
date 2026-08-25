# ApprovalNode ledger

`workflow_core::approval::ApprovalLedger` é a fronteira declarativa bounded para pausar um
workflow até uma decisão humana autenticada.

## Security contract

- cada request é ligado a `project_id`, `workflow_id`, `run_id`, `node_id` e `generation`;
- somente o `approver_id` registrado pode decidir;
- lifetime é fornecido pelo chamador e limitado pelo máximo declarado;
- allow emite token opaco de uso único; deny, expiry e cancel são terminais;
- binding divergente, actor divergente, replay e capacidade excedida falham fechado;
- o token não contém prompt, schema, argumentos ou segredos.

## Recovery semantics

A implementação desta fatia é em memória e bounded. Persistência, transporte de eventos,
identidade autenticada e UI humana são responsabilidades das camadas superiores. Após restart,
um pending approval não é automaticamente restaurado nem auto-concedido; a camada persistente
deverá reconstruir explicitamente o binding e respeitar a expiração antes de permitir novo pedido.
