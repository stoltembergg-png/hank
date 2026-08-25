# Spec: Skill lifecycle curator

> feature: skill-lifecycle-curator
> status: em implementação

## História

### US-653 — Orquestrar lifecycle com gates explícitos

Como mantenedor de Skills, quero uma decisão centralizada de lifecycle, para
que transições inválidas, ativação sem evidência e isolamento incorreto falhem
antes de qualquer mutação.

#### AC-846 — Transição legal produz decisão bounded

- **Dado** Skill project-scoped, identidade válida e transição legal
- **Quando** o curator decide
- **Então** retorna Allowed com versão e digest determinísticos.

#### AC-847 — Ativação exige evidências e rollback

- **Dado** target Active
- **Quando** faltar validação, avaliação, teste autônomo, autorização ou rollback
- **Então** retorna Denied com razão bounded e sem mutação.

#### AC-848 — Transições inválidas, escopo cruzado e repetição são seguros

- **Dado** transição ilegal, Skill de outro projeto ou estado já aplicado
- **Quando** o curator decide
- **Então** nega, rejeita ou retorna AlreadyApplied idempotentemente.

## Fora de escopo

- Persistência, eventos, concorrência, crash recovery, cache e alteração de
  ponteiro ativo.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-849 | O curator puro é a primeira fronteira antes do serviço transacional. | confirmada | Integração com repository e eventos será posterior. |

## Perguntas em aberto

Nenhuma.
