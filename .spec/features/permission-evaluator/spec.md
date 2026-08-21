# Spec: Permission evaluator

> feature: permission-evaluator
> status: implementada

## Contexto

PR-099 centraliza a decisão de autorização antes de qualquer execução de tool, impedindo bypass por descrição, capability ausente, projeto ausente ou budget indisponível.

## Histórias

### US-605 — Decisão de permissão fail-closed

Como runtime de tools, quero uma decisão determinística e auditável antes do handler, para que cada chamada respeite policy, projeto, capability, efeito e budget.

#### AC-632 — Matriz allow/ask/deny e validação de identidade

- **Dado** um request de tool com projeto, identidade, capability, efeito, policy e budget
- **Quando** o evaluator decide
- **Então** deny, identidade ausente, capability ausente ou budget indisponível falham fechadamente; leitura permitida pode prosseguir; efeitos sensíveis exigem confirmação conforme policy

#### AC-633 — Confirmação ask_once isolada

- **Dado** uma confirmação `ask_once` para projeto, tool, versão e capability
- **Quando** a mesma chamada é repetida ou outro projeto tenta usá-la
- **Então** somente a mesma chave scoped reutiliza a aprovação; `ask_every_time` nunca reutiliza

#### AC-634 — Concorrência e limpeza scoped

- **Dado** múltiplas avaliações concorrentes e aprovações em cache
- **Quando** threads avaliam ou um projeto é limpo
- **Então** não há corrida/panic e `clear_project` remove apenas aprovações daquele projeto

## Fora de escopo

- UI final de confirmação, execução de handlers, sandbox OS, armazenamento de secrets e policy de provider.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-607 | `ToolContext::PolicyDecision` é o contrato público existente para policy básica. | confirmada | Reutilizado sem alterar enum ou consumidores. |
| ASM-608 | O cache bounded em memória é suficiente para `ask_once` nesta camada. | confirmada | Limite explícito de 1024 aprovações; persistência fica fora do card. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-605 | A aprovação humana deve persistir entre processos? | respondida | Não neste card; o cache é scoped e em memória, persistência futura terá contrato próprio. |
