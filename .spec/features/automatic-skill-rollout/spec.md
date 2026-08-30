# Spec: automatic skill rollout

> feature: automatic-skill-rollout
> status: auditada

### US-1369 — Bounded staged activation

Como sistema, quero ativar uma skill apenas em escopo canary autorizado e com evidências vinculadas.

#### AC-1369 — Eligibility and scope

- **Dado** proposal, evaluation, regression, score e rollback válidos para o mesmo candidate.
- **Quando** a elegibilidade é avaliada.
- **Então** o canary é limitado ao projeto autorizado e à versão pinada.
- **Dado** evidence ausente/NoGo ou escopo acima da policy.
- **Quando** avaliado.
- **Então** a ativação é bloqueada.

### US-1370 — Stop and rollback boundary

Como sistema, quero interromper a ativação quando a health window falhar.

#### AC-1370 — Reversible activation

- **Dado** health failure ou kill switch.
- **Quando** o rollout é avaliado.
- **Então** retorna `Stopped`, exige rollback e não expande escopo.
- **Dado** health válida.
- **Quando** a janela termina.
- **Então** retorna `CanaryReady`, sem ativação global automática.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Elegibilidade staged, bounded e reversível, sem efeitos de ativação no runtime.
