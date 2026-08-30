# Spec: self-development PR

> feature: self-development-pr
> status: auditada

### US-1375 — Review-bound PR proposal

Como sistema, quero preparar uma PR draft rastreável sem torná-la efetiva automaticamente.

#### AC-1375 — Evidence-bound draft

- **Dado** candidate, issue, branch, base/head/tree e evidências de proposal, evaluation, regression e rollback válidos.
- **Quando** o draft é criado.
- **Então** o payload vincula toda a identidade, mantém `draft=true` e exige revisão/CI.
- **Dado** qualquer evidência obrigatória ausente.
- **Quando** o draft é solicitado.
- **Então** a criação é bloqueada.

### US-1376 — Stale and duplicate safety

Como sistema, quero invalidar evidência stale e atualizar a mesma proposta em duplicatas.

#### AC-1376 — Exact identity and no approval

- **Dado** novo head SHA ou tree diferente.
- **Quando** o status é verificado.
- **Então** retorna `Stale`.
- **Dado** identidade igual.
- **Quando** a proposta é repetida.
- **Então** retorna a mesma chave idempotente.
- **Então** o payload nunca representa aprovação, merge, release ou activation.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Contrato puro de proposta de PR draft, bounded, idempotente e fail-closed.
