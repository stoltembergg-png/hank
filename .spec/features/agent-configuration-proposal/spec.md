# Spec: agent configuration proposal

> feature: agent-configuration-proposal
> status: auditada

### US-1359 — Typed configuration diff

Como avaliador, quero propor alterações de configuração vinculadas a agent/version/candidate sem mutar a configuração ativa.

#### AC-1359 — Precedence and preservation

- **Dado** configuração ativa e diff typed válido.
- **Quando** a proposal é criada.
- **Então** preserva a configuração anterior, classifica precedence e produz digest estável.
- **Dado** alteração de system/security instruction.
- **Quando** a proposal é criada.
- **Então** é bloqueada e não altera a configuração ativa.

### US-1360 — Capability and budget boundary

Como sistema, quero impedir elevação silenciosa de capability, autonomia ou budget.

#### AC-1360 — Explicit policy delta

- **Dado** capability, autonomy ou budget delta sem aprovação explícita.
- **Quando** a proposal é criada.
- **Então** é bloqueada e não pode ativar.
- **Dado** proposal segura.
- **Quando** consultada.
- **Então** continua proposal-only e provider-neutral, sem credenciais.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Proposal typed, bounded, reversível e determinística; configuração ativa permanece intacta.
