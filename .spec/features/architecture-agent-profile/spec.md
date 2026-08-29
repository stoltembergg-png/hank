# Spec: architecture agent profile

> feature: architecture-agent-profile
> status: auditada

## Contexto

PR-212 define um perfil advisory para verificar o grafo arquitetural normativo,
boundaries, impacto de ADR/documentação e evidência de dependências. O contrato
não altera arquitetura nem substitui decisão humana, policy ou gate autoritativo.

## Boundary e não-escopo

- `agent-core` contém apenas schema, parser tipado, validação determinística,
  findings e handoff.
- O profile não edita arquitetura, ratifica ADR, altera graph gates, executa
  source/tests/commands, acessa Git, filesystem, rede, providers ou secrets.
- Layers, edges, documentos, diffs e textos externos são dados não confiáveis;
  texto instruction-like não cria capability nem execução.
- Evidence conserva somente identidade, paths relativos, status, contagens e
  digests bounded; não armazena conteúdo bruto.

## Histórias

### US-1334 — Architecture manifest bounded

Como arquiteto, quero validar um manifesto tipado e bounded para detectar ciclos
e dependências proibidas sem conceder mutação.

#### AC-1334 — Grafo e boundary

- **Dado** um profile válido, mapping `Active` e manifest com layers/edges
  válidos.
- **Quando** o grafo é avaliado.
- **Então** edges permitidos passam e o permit permanece read-only.
- **Dado** edge proibido UI→storage/provider, ciclo, layer desconhecido ou
  manifest malformed.
- **Quando** o grafo é avaliado.
- **Então** a avaliação falha fechado com finding `Failed`/`Blocked` e não edita
  o grafo.

### US-1335 — Architecture evidence e impacto documental

Como sistema de qualidade, quero vincular findings a graph revision, ADR/docs e
SHA/tree/policy exatos sem promover prova stale ou ausente.

#### AC-1335 — Evidência exata

- **Dado** evidence `Passed` para graph, dependencies, documents e ADR impact,
  com identities e digests válidos.
- **Quando** o report é validado.
- **Então** ele é `Pass` e preserva referências exatas.
- **Dado** graph revision stale, SHA/tree/policy incorretos, ADR ausente,
  evidence missing/skipped/no-run/malformed ou digest ausente.
- **Quando** o report é validado.
- **Então** ele permanece `NoProof`/`Blocked` e nunca vira sucesso.

### US-1336 — Architecture finding handoff

Como sistema de revisão, quero encaminhar findings arquiteturais sem permitir
edição automática, ratificação ou bypass de gates.

#### AC-1336 — Handoff advisory

- **Dado** finding de ciclo, edge proibido, ADR ausente ou blocker de evidence.
- **Quando** o handoff é criado.
- **Então** ele preserva IDs, severity, status e digest, permanece advisory e
  não pode editar o grafo, ratificar ADR, alterar gate ou aprovar.
- **Dado** descrição ou artifact instruction-like.
- **Quando** é incorporado ao manifest.
- **Então** permanece dado não confiável e não cria execução/capability.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Observabilidade

O contrato expõe profile/schema/policy revision, graph revision, project/task/
repository/worktree/branch, SHA/tree, layer/edge/document/check IDs, status,
severity, contagens e digests. Não expõe source, comandos, prompts ou artifacts
brutos.

## Segurança

O architecture agent é advisory e provider-neutral. Graph gate, ADR ratification,
refactor, merge e protected-branch policy permanecem autoritativos fora deste
slice. Hypothesis, missing, stale, malformed e blockers nunca são promovidos a
`Pass`.

## Rollback

Remover o módulo, testes, docs e verificação ONP remove somente o contrato de
domínio; não altera arquitetura, gates, branches ou runtime externo.

## Definition of Done

Manifest, graph checks, document impact, findings e evidence handoff bounded
implementados em `agent-core`, testes positivos/negativos rastreáveis,
documentação e verify ONP passam sem conceder autoridade executável.
