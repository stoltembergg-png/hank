# Spec: plugin permissions

> feature: plugin-permissions
> status: auditada

### US-1395 — Explicit plugin capability authorization

Como plataforma, quero autorizar capabilities de plugin por identidade exata e ciclo de vida.

#### AC-1395 — Default deny and exact manifest binding

- **Dado** um pedido de plugin sem grant ou com digest/version/plugin diferentes
- **Quando** a permissão é avaliada
- **Então** o resultado é `Denied` ou erro stale, sem herdar permissões de MCP ou de outro projeto.

#### AC-1396 — Revoke and upgrade re-consent

- **Dado** um grant aprovado que é revogado ou um manifest upgrade que adiciona capability
- **Quando** o plugin tenta iniciar/usar a capability
- **Então** o pedido é negado até novo grant explícito, e a revogação permanece efetiva.

## Segurança

- Default deny; nenhuma capability implícita de browser, UI, filesystem, rede ou processo.
- Grant vinculado a plugin ID, digest, versão, projeto, agente, capability, ação e policy revision.
- Revoke é terminal para o vínculo atual; upgrade com capability nova exige re-consentimento.

## Suposições

- ASM-1395: o Permission Engine de plugin permanece puro; lifecycle/adapters apenas consomem sua decisão.

## Perguntas em aberto

Nenhuma.
