# Spec: MCP tool discovery

> feature: mcp-tool-discovery
> status: auditada

### US-1385 — Validated staged discovery

Como sistema, quero descobrir tools MCP sem transformá-las em tools ativas.

#### AC-1385 — Manifest validation and staging

- **Dado** transport/server autorizado e manifest dentro dos limites.
- **Quando** `list-tools` é processado.
- **Então** os entries são ordenados e staged como `Pending`/`Disabled`, sem execução.
- **Dado** transport/server não autorizado, manifest oversized, schema inválido ou capability desconhecida.
- **Quando** processado.
- **Então** retorna erro tipado fail-closed.

### US-1386 — Refresh cannot widen trust

Como sistema, quero atualizar discovery sem ampliar permissões.

#### AC-1386 — Duplicate and stale safety

- **Dado** tool duplicada ou revision stale.
- **Quando** o manifest é processado.
- **Então** a operação é rejeitada ou permanece bounded.
- **Dado** refresh com nova lista.
- **Quando** processado.
- **Então** nenhum entry fica ativo nem ganha capability de execução automaticamente.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Discovery MCP validado, determinístico, não-executante e staged; sem UI ou activation.
