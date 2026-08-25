# Spec: missed-run policy

> feature: missed-run-policy
> status: em-implementacao

### US-1230 — Classificar ocorrências perdidas sem storm

Como scheduler, quero classificar ocorrências perdidas após downtime com cap e janela, para evitar
storm e manter uma decisão auditável.

#### AC-1231 — Policy determinística e bounded
- **Dado** downtime curto/longo, intervalo e cap explícitos
- **Quando** a policy é avaliada em um relógio de referência
- **Então** o resultado é determinístico, não excede o cap e classifica skip, catch-up, coalesce ou pause.

#### AC-1232 — Disabled e clock skew
- **Dado** job disabled ou relógio anterior ao due-at
- **Quando** o replay é avaliado
- **Então** nenhuma ocorrência é encaminhada e o resultado é fail-closed.

#### AC-1233 — Outcomes persistidos
- **Dado** uma decisão de missed occurrence
- **Quando** ela é registrada duas vezes
- **Então** o outcome é project-scoped e idempotente por ocorrência/action.

## Suposições
- ASM-1234: policy version `v1` é local e não altera a enum persistida de jobs nesta PR.

## Perguntas em aberto
Nenhuma.
