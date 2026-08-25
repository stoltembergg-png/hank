# Workflow scheduler integration

`WorkflowRunRequest::prepare` é a boundary entre scheduler e workflow persistence. Ela recebe IDs
tipados, revalida projeto e owner, carrega exatamente a versão solicitada e aceita somente status
`active`.

A request fixa `project_id`, `job_id`, `run_id`, `workflow_id`, `workflow_version`, `policy_ref` e uma
idempotency key determinística. Retries do mesmo job/run/version preservam essa chave.

Esta PR não executa nodes, não concede capabilities, não escolhe providers e não altera workflows.
A execução e o outcome terminal permanecem nas boundaries de execution/workflow existentes.
