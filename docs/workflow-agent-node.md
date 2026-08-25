# AgentNode adapter

`agent_runtime::agent_node::AgentNodeAdapter` é a fronteira provider-neutral entre o
runner de workflow e o `TurnInvoker` do Agent Runtime.

- `project_id` é transportado como scope opaco e comparado por igualdade exata;
- `AgentId` e `SessionId` permanecem IDs tipados;
- run/node/generation são bounded e preservados no resultado;
- cancellation é verificada antes do invoker;
- budget é validado antes do dispatch e contra usage depois do retorno;
- resultados de geração stale são rejeitados;
- providers concretos, credenciais, storage e UI não atravessam o adapter.
