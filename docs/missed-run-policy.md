# Missed-run policy

A policy `v1` classifica ocorrências perdidas usando relógio explícito, janela de atraso e cap de
catch-up. `Skip` produz decisões skip bounded; `CatchUp` produz no máximo `catch_up_cap` decisões;
`Coalesce` produz uma decisão com chave determinística; `Pause` produz uma decisão de pausa.

Jobs disabled e clocks anteriores ao `due_at` produzem zero decisões. Atrasos além da janela são
registrados como `skip` com razão `outside_window`, nunca como entrega garantida.

Cada decisão pode ser registrada em `scheduler_missed_outcomes`, escopada por projeto e idempotente
por `(project_id, run_id, occurrence_at_ms, action)`. O registro mantém action, reason, coalesce key,
policy version e timestamp para auditoria. Esta camada não executa providers, não altera permissões e
não implementa retries ou notifications.
