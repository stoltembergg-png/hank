# Scheduled job schema

A migração `0016_scheduler_jobs.sql` cria uma entidade project-scoped para jobs, sem iniciar
workers ou calcular próximas execuções.

Campos obrigatórios incluem `job_id`, `owner_id`, trigger, target versionado, timezone,
concurrency limit, missed-run policy, enabled/lifecycle e revision.

Triggers aceitos nesta camada:

- `one_shot` — timestamp positivo;
- `interval` — segundos >= 60;
- `cron` — expressão bounded armazenada como dado;
- `event` — nome bounded;
- `dependency` — job identity bounded.

Targets aceitos são workflow, agent ou tool com versão positiva. Lifecycle válido é `active`,
`disabled` ou `archived`; disabled e archived são dados explícitos, não execução implícita.
Updates exigem `expected_revision` e stale updates falham sem overwrite.

Cron parsing, cálculo de next-run, worker, scheduler loop, notificações e execução imediata
permanecem fora desta PR.
