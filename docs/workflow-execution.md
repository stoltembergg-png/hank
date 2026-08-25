# Workflow execution

`workflow-core::WorkflowRun` coordena um DAG já validado sem executar handlers.

- `run_id`, workflow ID/version e capacidade de nós em voo são bounded;
- `ready_nodes()` usa predecessores concluídos e ordenação lexical estável;
- `dispatch` aplica backpressure antes de mutar o node;
- `complete`, `fail`, `retry` e `cancel` preservam estados terminais;
- failure codes são limitados e armazenados sem payload bruto;
- `RetryEnvelope` transporta apenas identidade, tentativa, limite e razão redigida.

Esta camada não acessa SQLite, providers, tools, scheduler, rede ou UI. Adapters de
handlers e recuperação após crash são incrementos posteriores.
