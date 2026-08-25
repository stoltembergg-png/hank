# Spec: Group chat event contract

> feature: group-chat
> status: implementada

## História de usuário

### US-929 — Renderizar estado de group chat via eventos

Como UI de colaboração, quero consumir eventos project/session-scoped com
provenance, status e limites, sem acessar DB ou executar instruções.

#### AC-930 — Eventos válidos preservam scope e provenance

- **Dado** evento do project/session atuais
- **Quando** é aplicado ao store
- **Então** a sequência e IDs de agent/trace são preservados; evento foreign é
  rejeitado.

#### AC-931 — Estados pending/denied/terminated são visíveis

- **Dado** eventos de policy/delegation/session
- **Quando** chegam em ordem
- **Então** o store preserva os estados e torna término terminal.

#### AC-932 — Conteúdo é escaped e bounded

- **Dado** output grande ou com markup/injection
- **Quando** é validado/renderizado
- **Então** output oversized é rejeitado, markup é escaped e o store é bounded.

## Fora de escopo

- tela visual completa, DB, provider, tool call, invocation bypass e storage de
  secrets.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-933 | API/event bridge existente entregará eventos já autorizados. | confirmada | Esta fatia valida apenas contrato frontend e não cria bridge. |

## Perguntas em aberto

Nenhuma.
