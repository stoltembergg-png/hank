# Workflow persistence

A migration `0013_workflow_storage.sql` adiciona as tabelas versionadas
`workflow_definitions`, `workflow_nodes` e `workflow_edges`. Nodes e edges
possuem foreign key composta para a definição, impedindo órfãos; índices cobrem
project/version/status e ordering.

`SqliteWorkflowRepository::save_definition` valida o DAG antes da transação,
grava a definição completa em uma única transação e suporta optimistic
concurrency por `expected_version`. `load_definition` exige `project_id`,
reconstrói a topologia e valida novamente antes de retornar.

Não há executor, scheduler ou UI nesta fatia. Correções de schema são
forward-only; rollback operacional depende de backup/forward-fix.
