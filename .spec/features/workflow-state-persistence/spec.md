# Spec: workflow state persistence

> feature: workflow-state-persistence
> status: em-implementacao

### US-1032 — Persistir transições de workflow de modo atômico e replay-safe

Como runtime de workflow, quero persistir run/node state e journal de transições com escopo,
geração e idempotência para sobreviver a restart sem criar estados impossíveis ou repetir uma
transição já confirmada.

#### AC-1033 — Migração e run scope são bounded

- **Dado** banco limpo ou já migrado
- **Quando** a migração roda uma ou duas vezes e um run é criado
- **Então** as tabelas existem, o segundo run é rejeitado por identidade e o estado inicial é persistido.

#### AC-1034 — Compare-and-set é atômico

- **Dado** node state com geração e estado atuais
- **Quando** a transição esperada é aplicada
- **Então** journal, sequence e node state mudam na mesma transação; estado/generation divergentes falham sem overwrite.

#### AC-1035 — Replay e checkpoint são fail-closed

- **Dado** idempotency key repetida ou checkpoint com payload sensível
- **Quando** a transição é reapresentada
- **Então** o replay retorna o resultado anterior sem duplicar journal, e checkpoint aceita somente envelope bounded/redigido.

## Suposições

- ASM-1036: crash injection real e recovery policy completa pertencem à PR-187; esta PR prova atomicidade via rollback e replay no SQLite real.

## Perguntas em aberto

Nenhuma.
