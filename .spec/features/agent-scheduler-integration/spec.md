# Spec: agent scheduler integration

> feature: agent-scheduler-integration
> status: em-implementacao

### US-1260 — Preparar tarefa de agente bounded

Como scheduler, quero validar policy, budget, sessão e agente antes do dispatch, para que tarefas
agendadas não bypassem autonomy, permissions ou idempotency.

#### AC-1261 — Request válida e idempotente
- **Dado** agente ativo, policy permitida, budget disponível e session/run tipados
- **Quando** o scheduler prepara o dispatch
- **Então** produz request bounded com job/session/run/agent/project e chave idempotente determinística.

#### AC-1262 — Negativas fail-closed
- **Dado** agente disabled, autonomy negada ou budget esgotado
- **Quando** o scheduler prepara o dispatch
- **Então** falha antes da boundary de provider e não produz request executável.

#### AC-1263 — Capabilities e cancelamento
- **Dado** uma request preparada
- **Quando** ela é serializada para dispatch
- **Então** não concede capabilities e respeita cancelamento antes do envio.

## Suposições
- ASM-1264: criação persistente de sessão e execução provider permanecem nas boundaries existentes; esta PR prepara o contrato e não chama provider.

## Perguntas em aberto
Nenhuma.
