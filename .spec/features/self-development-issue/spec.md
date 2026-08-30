# Spec: self-development issue

> feature: self-development-issue
> status: auditada

### US-1371 — Traceable issue handoff

Como sistema, quero produzir um payload bounded para handoff humano de uma melhoria bloqueada.

#### AC-1371 — Identity and idempotency

- **Dado** candidate, evidence, repository, SHA e policy válidos.
- **Quando** o payload é criado.
- **Então** inclui decisão, risco, próximo gate e chave idempotente determinística.
- **Dado** solicitação duplicada.
- **Quando** serializada.
- **Então** a chave permanece igual para atualizar a mesma issue.

### US-1372 — Safe redacted handoff

Como sistema, quero impedir prompt injection, secrets e publicação sem policy.

#### AC-1372 — Fail-closed issue proposal

- **Dado** texto hostil ou secret-like.
- **Quando** o payload é criado.
- **Então** o conteúdo é escapado/redacted e nunca interpretado como instrução.
- **Dado** policy negada ou identidade stale.
- **Quando** solicitado.
- **Então** nenhum payload de issue é produzido.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Payload de handoff bounded, redacted e idempotente, sem publicação ou mutação externa.
