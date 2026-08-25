# Workflow entity

`workflow-core` define `Workflow` com workflow ID, project ID, owner ID, nome,
versão, policy reference, schema version, metadata ordenada e lifecycle:
`draft`, `active`, `paused`, `archived` e `blocked`.

O schema rejeita versões desconhecidas, metadata oversized, nomes/policies
inválidos e transições ilegais. A entidade não cria nodes, não acessa storage,
não agenda e não executa handlers.
