# Spec: scheduled job entity

> feature: scheduled-job-entity
> status: em-implementacao

### US-1120 — Definir jobs agendáveis persistíveis e bounded

Como owner de um projeto, quero registrar um job com trigger, target, timezone, concorrência e
lifecycle explícitos sem iniciar execução.

#### AC-1121 — Triggers e target válidos
- **Dado** trigger one-shot/interval/cron/event/dependency e target versionado
- **Quando** o job é criado
- **Então** o schema aceita tipos conhecidos e rejeita trigger/target ambíguos ou de outro projeto.

#### AC-1122 — Limites e lifecycle
- **Dado** frequência, concorrência e missed-run policy
- **Quando** são persistidos
- **Então** zero/valores abaixo do mínimo, excesso e lifecycle inválido falham; disabled/archived são explícitos.

#### AC-1123 — Revisão e migração
- **Dado** job existente
- **Quando** migrations repetem ou update usa revisão stale
- **Então** migration é idempotente e stale update não sobrescreve o job.

## Suposições
- ASM-1124: cron é armazenado como dado validado bounded; parsing/calculation é PR-192+.

## Perguntas em aberto
Nenhuma.
