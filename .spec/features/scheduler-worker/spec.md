# Spec: scheduler worker

> feature: scheduler-worker
> status: em-implementacao

### US-1220 — Despertar e encaminhar runs persistentes

Como operador do scheduler, quero um worker bounded que reclame runs devidos e publique um envelope
idempotente, para que o executor existente possa continuar o lifecycle fora desta camada.

#### AC-1221 — Tick bounded e dispatch
- **Dado** runs due e um limite de claims por tick
- **Quando** o worker executa um tick
- **Então** reclama no máximo o limite e publica envelopes com `project_id`, `run_id`, `job_id` e chave idempotente.

#### AC-1222 — Lease e shutdown
- **Dado** um run claimed pelo worker
- **Quando** o lease é renovado ou o operador solicita shutdown
- **Então** a renovação mantém o owner e ticks posteriores são rejeitados sem reclamar novos runs.

## Suposições
- ASM-1223: o consumer do `DispatchEnvelope` e a execução real permanecem nas boundaries existentes e em PRs posteriores.

## Perguntas em aberto
Nenhuma.
