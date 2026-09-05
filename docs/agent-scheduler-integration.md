# Agent scheduler integration

`AgentDispatchRequest::prepare` é a boundary bounded para tarefas agendadas de agente. Ela exige
IDs tipados de projeto, agente e sessão; valida `job_id`/`run_id`; aplica autonomia, budget e
cancelamento antes de qualquer provider boundary.

A request fixa uma idempotency key por projeto/run e limita `max_tokens` a 1.000.000 e ao budget
restante. Nenhuma capability é concedida pelo scheduler; tool calls e provider execution continuam
passando pelas APIs autorizadas existentes.

`AgentDispatchGate` aplica `security-core::rate_limit` à classe `Trigger` usando os IDs tipados de
agente e projeto. `prepare` só retorna `AgentDispatchRequest` depois de uma decisão permitida;
excesso retorna `RateLimited` com `retry_after_ms`, sem produzir dispatch. Recovery e métricas
usam buckets próprios bounded e não são bypasses ilimitados.

Esta PR não cria sessão persistente, não seleciona provider e não abre caminho de self-modification.
Essas operações permanecem nas boundaries de execução e serão integradas em incrementos posteriores.
