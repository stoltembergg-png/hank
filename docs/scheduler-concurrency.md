# Scheduler concurrency protection

`SchedulerConcurrency` fornece admission atômica por `(project_id, concurrency_key)` com limite
máximo 64. A transação remove leases expirados, conta somente admissions ativas e insere o novo
slot; dois workers concorrentes não ultrapassam o limite.

A admission é owner-bound. Um owner diferente não consegue liberar o slot ativo. Após expiry o slot
pode ser reutilizado; release explícito remove o slot para cancelamento ou outcome terminal.

A chave é recebida de uma boundary autorizada e não é interpretada a partir do payload do job. O
módulo não implementa lock distribuído, rate limiting ou autoscaling. Chaves de escopo diferente,
incluindo projetos diferentes, não colidem.
