# Spec: AgentNode adapter

> feature: workflow-agent-node
> status: implementada

### US-969 — Invocar AgentNode por boundary provider-neutral

Como executor de workflow, quero entregar um AgentNode ao Agent Runtime para preservar
identidade, budget, cancelamento e correlação sem acoplar provider ao workflow-core.

#### AC-970 — Identidade e contexto são validados antes do invoker

- **Dado** um request com project, agent, session, run e node bounded
- **Quando** a identidade diverge ou a requisição está cancelada
- **Então** o adapter rejeita antes do invoker e não produz efeito de execução.

#### AC-971 — Resultado determinístico preserva correlação e budget

- **Dado** um invoker determinístico
- **Quando** o AgentNode é executado dentro do budget
- **Então** o resultado preserva run/node/session/generation e usage; excedente de tokens falha fechado.

#### AC-972 — Geração stale não atravessa a fronteira

- **Dado** um resultado de uma geração anterior
- **Quando** ele é aceito contra a geração atual
- **Então** o adapter rejeita com erro stale sem alterar o resultado.

## Suposições

- ASM-973: o adapter coordena o handoff; provider concreto, storage e UI permanecem no runtime existente.

## Perguntas em aberto

Nenhuma.
