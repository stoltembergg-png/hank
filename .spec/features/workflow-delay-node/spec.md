# Spec: DelayNode plan

> feature: workflow-delay-node
> status: em-implementacao

### US-1003 — Modelar espera monotônica e cancelável

Como executor de workflow, quero representar uma espera bounded com deadline monotônico para
persistir estado e retomar sem bloquear worker.

#### AC-1004 — Fake clock determina deadline

- **Dado** um instante monotônico, duração válida e limite aprovado
- **Quando** o plano é consultado com um novo instante
- **Então** transita de `waiting` para `ready` somente ao atingir o deadline.

#### AC-1005 — Limites e estados terminais são explícitos

- **Dado** duração zero, duração acima do limite ou cancelamento
- **Quando** o plano é criado/cancelado
- **Então** zero fica pronto imediatamente, excesso falha e cancelamento é terminal/idempotente.

#### AC-1006 — Pause/resume não altera o relógio externo

- **Dado** um delay waiting
- **Quando** ele é pausado e retomado em instantes monotônicos
- **Então** o tempo restante é preservado, sem `sleep`, thread ou wall-clock.

## Suposições

- ASM-1007: o relógio monotônico e a persistência/recovery serão fornecidos por camadas superiores; esta fatia só mantém o contrato em memória.

## Perguntas em aberto

Nenhuma.
