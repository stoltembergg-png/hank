# Spec: MCP settings UI

> feature: mcp-settings-ui
> status: auditada

### US-1387 — Safe MCP settings

Como usuário, quero inspecionar e configurar servidores MCP por API typed.

#### AC-1387 — Validation and redacted rendering

- **Dado** endpoint malformed, capability inválida ou origin não local.
- **Quando** o formulário é validado.
- **Então** a operação é rejeitada antes do IPC.
- **Dado** server/tool text hostil ou secret-like.
- **Quando** renderizado.
- **Então** é escapado/redacted e não vira HTML/instrução.

### US-1388 — Review and revoke lifecycle

Como usuário, quero revogar grants e revisar tools staged sem ativá-las silenciosamente.

#### AC-1388 — Typed actions and staged trust

- **Dado** grant autorizado.
- **Quando** revoke é acionado.
- **Então** chama apenas o comando typed de revogação com escopo explícito.
- **Dado** tool staged.
- **Quando** exibida.
- **Então** permanece `Pending/Disabled` até aprovação explícita.
- **Dado** resposta stale.
- **Quando** recebida.
- **Então** não restaura grant ou estado ativo.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Settings MCP acessível, typed, redacted e stale-safe, sem SQLite/invoke genérico ou execution console.
