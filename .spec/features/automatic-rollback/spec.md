# Spec: automatic rollback

> feature: automatic-rollback
> status: auditada

### US-1367 — Last-known-good recovery

Como sistema, quero restaurar a versão anterior após um trigger crítico.

#### AC-1367 — Atomic and idempotent rollback

- **Dado** versão ativa e last-known-good válido.
- **Quando** ocorre crash, regressão ou revogação.
- **Então** o ponteiro anterior é restaurado atomicamente e o candidate é quarantined.
- **Dado** rollback repetido.
- **Quando** executado.
- **Então** permanece idempotente e bloqueia novas ativações.

### US-1368 — Fail-closed recovery

Como sistema, quero impedir restauração insegura.

#### AC-1368 — No LKG and policy mismatch

- **Dado** ausência de LKG ou policy revision incompatível.
- **Quando** rollback é solicitado.
- **Então** retorna `Blocked` sem mutação.
- **Dado** estado recuperado.
- **Quando** consultado.
- **Então** registra trigger, versões, epoch e quarantine, sem rollout.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Rollback bounded, atômico, idempotente e fail-closed, sem rollout ou mutação destrutiva externa.
