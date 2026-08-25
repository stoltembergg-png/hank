# Spec: AgentGroup session

> feature: agent-group-session
> status: implementada

## História de usuário

### US-871 — Criar sessão multi-agent bounded

Como plataforma multi-agent, quero uma sessão com snapshot de membership,
rounds, budget, contexto referenciado e cancelamento, para que a colaboração
não misture identidade ou limites com sessões individuais.

#### AC-872 — Sessão captura identidade e membership snapshot

- **Dado** um AgentGroup válido
- **Quando** uma sessão é criada
- **Então** project/group/trace e membership são copiados para snapshot estável.

#### AC-873 — Round e budget interrompem progresso

- **Dado** sessão ativa com limites
- **Quando** rounds ou tokens excedem os limites
- **Então** novo progresso é rejeitado sem execução externa.

#### AC-874 — Cancelamento é terminal e idempotente

- **Dado** sessão criada ou ativa
- **Quando** cancel é aplicado e repetido
- **Então** fecha em `Cancelled`, repete sem mutação e rejeita novo round.

## Fora de escopo

- Persistence, streaming/event bus, invocation, delegation, moderator,
  synthesis, tools e UI.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-875 | Session entity pode viver em `agent-core` até storage/event boundary posterior. | confirmada | Mantida arquitetura sem criar crate inexistente. |

## Perguntas em aberto

Nenhuma.
