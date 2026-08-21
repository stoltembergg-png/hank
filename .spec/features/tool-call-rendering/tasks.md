# Tasks: Tool-call rendering

> feature: tool-call-rendering
> status: implementada

## T-653 — Implementar card bounded de tool call [concluida]

- Refs: US-615, AC-662, AC-663, AC-664
- Arquivos: frontend/src/chat/tool-call/ToolCallCard.tsx, frontend/src/chat/tool-call/ToolCallCard.css, frontend/src/chat/ChatPage.tsx, frontend/src/chat/ChatPage.css
- Notas: estados typed, metadata de escopo/trace, serialização bounded, redaction de chaves/valores sensíveis e callback de aprovação sem execução local.

## T-654 — Cobrir apresentação e integração do read model [concluida]

- Refs: US-615, AC-662, AC-663, AC-664
- Arquivos: frontend/tests/tool-call-card.test.tsx, frontend/tests/chat-page.test.tsx
- Notas: testes verificam estados, limites, redaction, conteúdo tratado como texto, ausência de execução em `denied` e propagação de aprovação em `ask`.
