# Spec: scheduler persistence

> feature: scheduler-persistence
> status: em-implementacao

### US-1200 — Persistir runs e leases do scheduler

Como scheduler, quero armazenar estado de runs e leases no SQLite, para que restart e múltiplos
processos tenham uma autoridade durável.

#### AC-1201 — Migration e isolamento
- clean/upgrade migration cria `scheduler_runs`, colunas bounded de due/status e índices;
- segunda execução da migration não duplica schema;
- consultas exigem project scope.

#### AC-1202 — Atomic claim e lease expiry
- dois claimers não obtêm o mesmo run;
- claim com lease expirado pode ser recuperado atomicamente;
- owner incorreto não pode completar o run.

#### AC-1203 — Completion
- somente o lease owner completa o run;
- completion é terminal e bounded;
- estado permanece após novo pool/restart.

## Suposições
- ASM-1204: worker, polling e notificações permanecem fora desta PR.

## Perguntas em aberto
Nenhuma.
