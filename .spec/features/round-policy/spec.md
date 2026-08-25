# Spec: Round policy

> feature: round-policy
> status: implementada

## História de usuário

### US-918 — Encerrar rounds bounded

Como sessão de grupo, quero limites explícitos de rounds e turns, para que
no-progress, budget, cancelamento e retries não produzam loops operacionais.

#### AC-920 — Limite de rounds é exato

- **Dado** policy com máximo configurado
- **Quando** rounds são iniciados
- **Então** rounds até o limite passam e o seguinte torna-se terminal.

#### AC-921 — No-progress e término explícito são terminais

- **Dado** dois turns consecutivos sem progresso ou stop por budget/cancel
- **Quando** a policy registra o evento
- **Então** não aceita nova atividade.

#### AC-922 — Retry de turn não incrementa

- **Dado** turn ID já registrado
- **Quando** retry é recebido
- **Então** retorna duplicate sem alterar contadores ou scope.

## Fora de escopo

- synthesis, scheduler, workflow loops, UI e model provider.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-923 | No-progress é definido como dois turns consecutivos sem progresso. | confirmada | Constante bounded na policy. |

## Perguntas em aberto

Nenhuma.
