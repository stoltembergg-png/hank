# Spec: CI status integration

> feature: ci-status-integration
> status: auditada

## Contexto

PR-215 define a leitura declarativa de resultados de CI vinculados a PR/event/SHA/tree/policy. O domínio classifica evidência; adapters externos fazem API autenticada.

## Histórias

### US-1343 — CI evidence identity

Como agente, quero consumir somente contexts allowlisted ligados ao SHA/tree/policy corretos.

#### AC-1343 — Matrix fail-closed

- **Dado** contexto allowlisted PASS/FAIL com identidade exata.
- **Quando** o status é avaliado.
- **Então** o resultado é classificado deterministicamente.
- **Dado** missing, duplicate, skipped, cancelled, timeout, malformed ou identidade divergente.
- **Quando** o status é avaliado.
- **Então** o resultado é `unknown`/`blocked`, nunca PASS.

### US-1344 — Merge group

Como sistema de integração, quero usar a identidade do evento `merge_group`, sem reutilizar cache de outro SHA.

#### AC-1344 — Event binding

- **Dado** evento `merge_group` com head/tree/policy próprios.
- **Quando** os checks são avaliados.
- **Então** somente evidências desse evento são aceitas; policy N/A permanece explícita.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Contrato puro, matriz de testes, documentação e verify ONP; nenhum acesso GitHub/rede/cache/segredo e nenhuma autorização de merge.
