# Spec: MCP permission integration

> feature: mcp-permission-integration
> status: auditada

### US-1383 — Scoped MCP authorization

Como Permission Engine, quero autorizar capabilities MCP por escopo exato e ação.

#### AC-1383 — Default deny and scope isolation

- **Dado** request de discovery ou execution sem grant.
- **Quando** avaliado.
- **Então** retorna `Denied` sem efeito.
- **Dado** grant para server A/tool B/origin O/project P/agent G.
- **Quando** request usa escopo diferente.
- **Então** continua `Denied`.
- **Dado** grant de discovery.
- **Quando** usado para execution.
- **Então** não autoriza execution.

### US-1384 — Grant lifecycle and replay resistance

Como Permission Engine, quero limitar a duração e impedir replay de decisões.

#### AC-1384 — Expiry, revoke and stale decisions

- **Dado** grant one-shot, session ou persistent.
- **Quando** expira ou é revogado.
- **Então** a próxima avaliação retorna `Denied`.
- **Dado** request ID já avaliado ou policy revision stale.
- **Quando** reavaliado.
- **Então** retorna erro tipado sem ampliar permissão.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Permissões MCP default-deny, scoped, bounded e auditáveis; sem discovery/UI/credential storage.
