# Spec: workflow scheduler integration

> feature: workflow-scheduler-integration
> status: em-implementacao

### US-1250 — Resolver workflow agendado por versão

Como scheduler, quero resolver uma versão ativa de workflow e produzir uma request correlacionada,
para que a execução existente receba identidade e policy sem bypass.

#### AC-1251 — Resolve e idempotência
- **Dado** workflow ativo, projeto e owner compatíveis
- **Quando** o scheduler prepara o dispatch
- **Então** a request fixa `workflow_version`, `job_id`, `run_id` e uma chave idempotente determinística.

#### AC-1252 — Rejeições de segurança
- **Dado** workflow arquivado, versão inexistente, projeto divergente ou owner divergente
- **Quando** o scheduler prepara o dispatch
- **Então** falha sem produzir request executável.

#### AC-1253 — Retry preserva identidade
- **Dado** o mesmo job/run/version
- **Quando** o dispatch é preparado novamente
- **Então** a request mantém a mesma idempotency key e não concede capabilities.

## Suposições
- ASM-1254: execução, budgets e outcome terminal permanecem nas APIs de workflow/execution existentes e serão conectados em incremento posterior.

## Perguntas em aberto
Nenhuma.
