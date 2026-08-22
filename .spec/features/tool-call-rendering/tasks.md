# Tasks: Tool-call rendering

> feature: tool-call-rendering
> status: implementada

## T-653 — Implementar componentes ToolCall states [concluida]

- Refs: US-615, AC-662, AC-663, AC-664
- Arquivos: `frontend/src/components/ToolCall/`
- Notas: estados visuais, argumentos redigidos, approval affordance, React text escaping, output bounded/truncation marker.

## T-654 — Integrar ToolCall no ChatPage [concluida]

- Refs: US-615, AC-662, AC-663, AC-665
- Arquivos: `frontend/src/chat/ChatPage.tsx`
- Notas: recebe tool calls via Application API props, filtra por project/agent da sessão, sem execução local.

## T-655 — Testes component/unit + XSS/prompt-injection fixtures [concluida]

- Refs: US-615, AC-662, AC-663, AC-664, AC-665
- Arquivos: `frontend/tests/tool_call_rendering_ac_tests.test.tsx`, `frontend/tests/chat-page.test.tsx`
- Notas: estados, redaction, approval/denied, XSS text-only, helper escaping, project isolation.

## T-656 — Documentar boundary ToolCall rendering [concluida]

- Refs: US-615, AC-662, AC-663, AC-664, AC-665
- Arquivos: `docs/tool-call-rendering.md`
- Notas: estados, semântica de approval, redaction, limites visuais, isolamento de projeto.