# Spec: improvement scoring

> feature: improvement-scoring
> status: auditada

### US-1365 — Explainable deterministic score

Como avaliador, quero combinar métricas com policy versionada e precisão bounded.

#### AC-1365 — Stable weighted score

- **Dado** métricas válidas e pesos fixos da policy.
- **Quando** o score é calculado.
- **Então** o resultado é determinístico, explicável e vinculado à policy/evidence.
- **Dado** métrica ausente ou unknown.
- **Quando** o score é calculado.
- **Então** a classe é `Unknown`.

### US-1366 — Hard blocker precedence

Como sistema, quero que falhas críticas não sejam compensadas por ganhos locais.

#### AC-1366 — Fail-closed decision

- **Dado** falha de segurança/regressão, evidence stale ou policy ausente.
- **Quando** o score é calculado.
- **Então** a classe é `NoGo`, independentemente da pontuação.
- **Dado** score válido.
- **Quando** consultado.
- **Então** não ativa rollout e não aceita pesos do candidate.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Score determinístico, bounded, explicável, versionado e sem efeitos de ativação.
