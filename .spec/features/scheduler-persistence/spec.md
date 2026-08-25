# Spec: scheduler persistence

> feature: scheduler-persistence
> status: em-implementacao

### US-1200 — Persistir runs e leases do scheduler

Como scheduler, quero armazenar estado de runs e leases no SQLite, para que restart e múltiplos
processos tenham uma autoridade durável.

#### AC-1201 — Migration e isolamento
- **Dado** um banco limpo ou já atualizado
- **Quando** as migrations são executadas uma ou mais vezes e uma run é consultada
- **Então** `scheduler_runs`, colunas bounded, índices e project scope existem sem duplicação;

#### AC-1202 — Atomic claim e lease expiry
- **Dado** dois claimers ou um lease expirado
- **Quando** tentam reclamar o mesmo run
- **Então** somente um lease vigente é aceito, e lease expirado pode ser recuperado atomicamente;

#### AC-1203 — Completion
- **Dado** um run claimed por um owner
- **Quando** o owner ou outro actor tenta completar o run
- **Então** somente o owner completa, o estado vira terminal e permanece após restart;

## Suposições
- ASM-1204: worker, polling e notificações permanecem fora desta PR.

## Perguntas em aberto
Nenhuma.
