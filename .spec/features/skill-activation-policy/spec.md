# Spec: Governed Skill activation policy

> feature: skill-activation-policy
> status: em implementação

## História

### US-651 — Autorizar ativação somente com policy e evidência

Como mantenedor de Skills, quero uma decisão explícita de ativação, para que
nenhum candidato altere o ponteiro ativo sem autonomia, aprovação e evidência.

#### AC-836 — Autonomia suficiente permite decisão sem mutação

- **Dado** policy L3/L4 e quatro digests de evidência válidos
- **Quando** a política decide a ativação
- **Então** retorna Allowed, decisão determinística e `active_pointer_changed=false`.

#### AC-837 — Autonomia inferior exige aprovação ou nega

- **Dado** policy L2, L1 ou L0
- **Quando** a decisão é solicitada
- **Então** L2 exige aprovação humana, aprovação explícita permite a decisão, e
  L1/L0 sem aprovação não ativam.

#### AC-838 — Evidência e identidade incompletas falham fechadas

- **Dado** digest ausente ou actor inválido
- **Quando** a política decide
- **Então** rejeita antes de qualquer mutação ou transição.

## Fora de escopo

- Persistir o ponteiro ativo, rollout, repository mutation, UI ou rollback.
- Aprovação automática, alteração de autonomia ou execução de candidato.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-839 | A decisão é uma boundary pura antes do repositório. | confirmada | A mutação e o rollback ficam em cards posteriores. |

## Perguntas em aberto

Nenhuma.
