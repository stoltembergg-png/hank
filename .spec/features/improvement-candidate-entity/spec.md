# Spec: improvement candidate entity

> feature: improvement-candidate-entity
> status: auditada

### US-1351 — Versioned candidate

Como avaliador, quero um candidato imutavelmente vinculado a observações, policy, owner e projeto.

#### AC-1351 — Provenance and draft state

- **Dado** candidate sem source observation, policy ou proposal reference.
- **Quando** criado.
- **Então** falha fechadamente.
- **Dado** candidate válido.
- **Quando** criado.
- **Então** inicia em `Draft`, com versão e digest determinísticos.

### US-1352 — Isolated lifecycle

Como sistema, quero permitir somente transições ordenadas e impedir cross-project authorization.

#### AC-1352 — Ordered lifecycle and isolation

- **Dado** candidate de outro projeto.
- **Quando** autorizado.
- **Então** é negado.
- **Dado** estado lifecycle.
- **Quando** uma transição fora da ordem é solicitada.
- **Então** é rejeitada e o candidate não é aprovado.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Entidade pura e bounded pronta para evaluator, sem aplicação de mudança ou ativação de runtime.
