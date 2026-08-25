# Workflow edges and DAG validation

`workflow-core` define `WorkflowEdge` com source/target nodes e ports, label de
condição apenas declarativa e ordering explícito. `WorkflowGraph` mantém nodes
em mapa determinístico e edges bounded.

A validação rejeita self-edges, referências desconhecidas, duplicatas, edges de
outro workflow, shapes oversized e ciclos por DFS. O algoritmo não executa
labels nem payloads e não adiciona persistência, scheduler ou execução paralela.
