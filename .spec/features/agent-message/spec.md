# Spec: Agent-to-agent message rendering

> feature: agent-message
> status: implementada

## História de usuário

### US-934 — Distinguir mensagens internas entre agents

Como usuário do group chat, quero ver sender, receiver, provenance e status sem
confundir conteúdo interno com instrução do sistema.

#### AC-935 — Provenance e status são visíveis sem action affordance

- **Dado** mensagem do mesmo project/session
- **Quando** é renderizada
- **Então** sender, receiver, trace, invocation, round e status são preservados
  e nenhuma ação é autorizada pelo renderer.

#### AC-936 — Isolation e dedupe são fail-closed

- **Dado** mensagem cross-project, identidade desconhecida ou message ID repetido
- **Quando** chega ao store
- **Então** é rejeitada sem duplicar estado.

#### AC-937 — Conteúdo é untrusted data

- **Dado** texto com markup/injection e estado error/terminal
- **Quando** é renderizado
- **Então** markup é escaped, trust marker permanece untrusted e status fica
  explícito.

## Fora de escopo

- criação de mensagens, delegation, tool call, DB, persistência ou inferência de
  trust a partir do texto.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-938 | A autorização de ações ocorre fora do renderer, na Application API. | confirmada | Renderer fixa `actionAllowed: false`. |

## Perguntas em aberto

Nenhuma.
