# Workflow state persistence

A migração `0014_workflow_state.sql` adiciona tabelas bounded para:

- `workflow_runs`: identidade project/run, workflow version, generation e sequence;
- `workflow_node_states`: estado, tentativa e checkpoints before/after;
- `workflow_transitions`: journal com idempotency key, sequence e recovery class;
- `workflow_pending_approvals` e `workflow_pending_delays`: anchors para retomada posterior.

`agent_runtime::workflow_state_repo::StateStore` aplica transições em uma única transação
SQLite usando compare-and-set de `state` e `generation`. Uma idempotency key repetida retorna
`Replayed` sem inserir novo journal; conflito não sobrescreve o row existente.

Checkpoints são opcionais, limitados a 4096 bytes e rejeitam chaves textuais de credencial,
password, secret ou token. O store não persiste prompts, provider payloads ou capabilities.

A PR não afirma recovery após crash: a policy de startup, leases e quarantine de efeitos
unknown pertence à PR-187. A migração é aditiva, repetível e não reescreve histórico aplicado.
