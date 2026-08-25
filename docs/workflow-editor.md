# Workflow editor

O editor usa `WorkflowEditorModel` como fronteira pura e bounded:

- nós e edges são mantidos em memória, separados do core e do storage;
- IDs, labels, número de nós e número de edges têm limites explícitos;
- self-edge, edge para nó ausente, edge duplicada e ciclo são rejeitados sem mutação;
- comandos carregam `project_id`, `workflow_id`, draft e `expected_version`;
- labels são escapados antes de entrar no comando/renderização;
- validate e save são APIs injetadas e tipadas;
- submit duplicado é rejeitado por chave estável;
- resposta stale/denied não altera o draft local;
- o componente React não acessa SQLite, Tauri, providers ou capabilities.

Salvar é sempre uma ação explícita. O editor não executa o workflow ao salvar e não persiste
conteúdo de prompt ou secrets no form state.
