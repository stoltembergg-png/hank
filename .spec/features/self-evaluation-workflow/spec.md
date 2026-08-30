# Spec: self-evaluation workflow

> feature: self-evaluation-workflow
> status: auditada

### US-1353 — Evaluate exact candidate snapshot

Como sistema, quero avaliar um candidate com snapshot exato, estágios bounded e decisão reproduzível.

#### AC-1353 — Required stages and identity

- **Dado** candidate com policy, tests, version e SHA exatos.
- **Quando** o workflow é iniciado.
- **Então** cria snapshot e exige os estágios validation, test e security.
- **Dado** candidate sem tests/policy ou SHA divergente.
- **Quando** o workflow é iniciado.
- **Então** produz `Blocked` sem aprovação.

### US-1354 — Crash and human boundary

Como operador, quero que crash/timeout não promova candidate e que aprovação seja externa.

#### AC-1354 — Durable decision

- **Dado** falha de evaluator ou estágio incompleto.
- **Quando** a decisão é produzida.
- **Então** permanece `Blocked`/`Rejected` e referencia candidate/version/SHA.
- **Dado** decisão `Approved`.
- **Quando** consultada.
- **Então** ainda não possui capacidade de ativação ou rollout.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Workflow/decision record puro, bounded, determinístico e fail-closed, sem rollout ou mutação.
