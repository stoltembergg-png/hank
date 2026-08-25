# PythonNode adapter

`agent_runtime::python_node::PythonNodeAdapter` é um boundary fino para o worker Python opcional.

- delega execução a `PythonExecutor`;
- não cria subprocessos, SDK, provider ou protocolo paralelo;
- preserva `PythonLifecycle`, `WorkerTransport`, schema versionado e limites existentes;
- rejeita cancellation antes do lifecycle/worker dispatch;
- converte cancellation posterior a um sucesso em outcome `Cancelled`;
- mantém operation key, trace e envelope do executor;
- o core continua compilando e testando sem runtime Python instalado.
