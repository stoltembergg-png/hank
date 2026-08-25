# Spec: Workflow entity

> feature: workflow-entity
> status: implementada

## História de usuário

### US-944 — Identificar e versionar workflows

Como camada de workflow, quero uma entidade persistível e project-scoped com
lifecycle explícito, para que nodes futuros possam referenciá-la sem executar
nada implicitamente.

#### AC-945 — Identity, lifecycle e round-trip são bounded

- **Dado** project/owner/name/policy válidos
- **Quando** workflow é criado, ativado e serializado
- **Então** IDs, versão, status e schema version são preservados em round-trip.

#### AC-946 — Versão, metadata e transições inválidas falham fechado

- **Dado** versão zero/regressiva, metadata oversized ou transição inválida
- **Quando** a entidade é modificada
- **Então** retorna erro tipado sem executar ou mutar indevidamente.

#### AC-947 — Schema desconhecido e cross-project não são aceitos

- **Dado** schema version desconhecido ou outro project ID
- **Quando** workflow é desserializado/associado
- **Então** falha fechado e não atravessa projeto.

## Fora de escopo

- nodes, edges, storage, scheduler, executor, editor e rollout autônomo.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-948 | `workflow-core` é o boundary inicial até existir storage-core de workflows. | confirmada | PR adiciona somente contrato in-process. |

## Perguntas em aberto

Nenhuma.
