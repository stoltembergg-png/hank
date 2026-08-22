# Spec: Tool-call rendering

> feature: tool-call-rendering
> status: implementada

## Contexto

PR-109 expõe componentes de UI para exibir tool calls, seus estados (pending/allowed/ask/denied/running/succeeded/failed/cancelled/timeout), decisões, resultados e erros no chat. A UI é apenas consumidora de eventos da API — sem execução local, sem acesso SQLite, sem bypass de confirmação.

## Histórias

### US-615 — Renderização de tool calls

Como usuário, quero ver cada tool call com seu estado, argumentos redigidos, decisão de permissão e vínculo de trace, para revisar autorização, efeitos, custos e falhas sem confiar no conteúdo como instrução.

#### AC-662 — Estados renderizados

- **Dado** tool call emitido via eventos internos
- **Quando** renderizo
- **Então** cada estado (pending/allowed/ask/denied/running/succeeded/failed/cancelled/timeout) tem componente visual distinto

#### AC-663 — Argumentos redigidos e approval affordance

- **Dado** args contendo segredos/prompt injection
- **Quando** renderizo
- **Então** segredos são redigidos visualmente; estado 'ask' mostra botão de approval; 'denied' não oferece execução

#### AC-664 — Output truncado e conteúdo malicioso

- **Dado** output truncado ou contendo HTML/script
- **Quando** renderizo
- **Então** truncamento permanece marcado; conteúdo malicioso é texto escapado (sem XSS/HTML injection)

#### AC-665 — Isolamento de projeto e sem execução

- **Dado** tool calls de múltiplos projetos/agentes
- **Quando** renderizo
- **Então** nenhum acesso SQLite/DB direto; UI só consome Application API/eventos; nenhuma execução local

## Fora de escopo

- Execução no frontend, acesso SQLite, bypass de confirmation, renderização automática de HTML/script ou UI completa de terminal.
- Lógica de negócio de autorização (já implementada no permission evaluator PR-099).

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-626 | Eventos de tool calls chegam via chat stream contract (PR-090) | confirmada | Evento `tool_call` com payload tipado |
| ASM-627 | Permission decisions chegam via events antes/durante execução | confirmada | Eventos `permission_decision` |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-615 | Precisa de animação para transição de estados? | respondida | Não no card; CSS transitions básicas suficientes |