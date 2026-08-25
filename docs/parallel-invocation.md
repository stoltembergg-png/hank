# Parallel invocation batch

`ParallelBatch` é um planner bounded, não um executor de providers. Ele aceita
somente children com cycle/depth preflight `Pass`, verifica project scope,
deduplica IDs e aplica limites de fan-out/concurrency.

Cancelamento é idempotente. O join valida todos os IDs e retorna outcomes na
ordem determinística da entrada, explicitando falhas/cancelamentos e evitando
órfãos. Nenhum worker ou scheduler implícito é iniciado.
