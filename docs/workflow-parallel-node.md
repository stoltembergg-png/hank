# ParallelNode planning contract

`workflow_core::parallel::ParallelPlan` modela o contrato bounded de ParallelNode sem executar
workers. Valida fan-out máximo (até 512), concorrência não nula, IDs únicos e join completo.

As políticas `all`, `any` e `quorum(n)` retornam `Satisfied`, `Failed` ou `Cancelled`. Os
resultados são sempre reordenados pela ordem declarada dos branches. Cancelamento é idempotente
e não cria tarefas órfãs. Scheduler, providers, execução distribuída e propagação externa
permanecem fora desta fatia.
