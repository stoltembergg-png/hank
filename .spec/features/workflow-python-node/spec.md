# Spec: PythonNode adapter

> feature: workflow-python-node
> status: em-implementacao

### US-981 — Encaminhar PythonNode ao worker opcional

Como executor de workflow, quero encaminhar um PythonNode ao executor Python existente,
para que Python permaneça opcional, versionado e isolado por worker protocol.

#### AC-982 — Boundary reutiliza executor e mantém Python opcional

- **Dado** um request PythonNode
- **Quando** o adapter o encaminha
- **Então** ele usa exclusivamente `PythonExecutor`/`PythonLifecycle`/`WorkerTransport`, sem subprocesso, provider ou SDK novo no workflow.

#### AC-983 — Capability, timeout, output e cancelamento são fail-closed

- **Dado** worker ausente, versão incompatível, output oversized, timeout ou cancellation
- **Quando** o PythonNode é executado
- **Então** o adapter propaga outcome tipado sem processo órfão, sucesso falso ou payload acima do limite.

#### AC-984 — Correlation é preservada

- **Dado** um request válido
- **Quando** o worker responde
- **Então** operation key, project/session/task/trace e outcome permanecem correlacionados no envelope.

## Suposições

- ASM-985: `PythonExecutor` e seus contratos de worker existentes são a autoridade de execução; o adapter apenas compõe essa API.

## Perguntas em aberto

Nenhuma.
