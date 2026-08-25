# Spec: Explicit Skill rollback

> feature: skill-rollback
> status: implementada

## História

### US-652 — Restaurar uma versão conhecida sem perder provenance

Como mantenedor de Skills, quero selecionar uma versão conhecida para rollback,
para recuperar uma ativação ruim de modo explícito, bounded e repetível.

#### AC-841 — Target conhecido produz decisão restaurável

- **Dado** active version, target conhecido e digest válido
- **Quando** o rollback é decidido
- **Então** retorna Applied com invalidation requerida, sem apagar provenance.

#### AC-842 — Repetição é idempotente

- **Dado** active version já igual ao target
- **Quando** rollback é solicitado novamente
- **Então** retorna AlreadyApplied sem nova mutação.

#### AC-843 — Target desconhecido ou identidade inválida falha fechada

- **Dado** target não conhecido ou digest/identidade inválidos
- **Quando** rollback é solicitado
- **Então** nega ou rejeita sem tocar estado ativo.

## Fora de escopo

- Transação SQLite, cache real, bindings, restart/recovery ou rollout.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-844 | A decisão antecede a operação transacional do repositório. | confirmada | Persistência e crash recovery ficam em fatia posterior. |

## Perguntas em aberto

Nenhuma.
