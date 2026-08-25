# DelayNode plan

`workflow_core::delay::DelayPlan` modela espera sem bloquear worker.

- usa ticks monotônicos fornecidos pelo chamador;
- aceita duração zero como `Ready` imediato;
- rejeita duração acima do limite e overflow de deadline;
- suporta `Waiting`, `Paused`, `Ready` e `Cancelled`;
- `pause` preserva o tempo restante e `resume` recalcula apenas deadline relativo;
- cancelamento é terminal e idempotente;
- não usa `sleep`, thread, scheduler ou wall-clock.

Persistência, recovery e integração com scheduler permanecem em camadas posteriores.
