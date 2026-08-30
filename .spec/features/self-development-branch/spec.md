# Spec: self-development branch

> feature: self-development-branch
> status: auditada

### US-1373 — Isolated development mapping

Como sistema, quero mapear uma issue autorizada a um único branch/worktree isolado.

#### AC-1373 — Fail-closed identity and policy

- **Dado** issue ausente, `NO_GO`, policy negada ou branch protegida.
- **Quando** uma sessão é solicitada.
- **Então** a criação é bloqueada.
- **Dado** issue, policy, candidate, versão e base SHA válidos.
- **Quando** a sessão é solicitada novamente.
- **Então** retorna o mesmo mapeamento determinístico.

### US-1374 — Safe lifecycle cleanup

Como sistema, quero limpar somente worktrees registrados e autorizados.

#### AC-1374 — Allowlist and orphan safety

- **Dado** root fora da allowlist ou worktree desconhecido.
- **Quando** a sessão/cleanup é avaliada.
- **Então** é bloqueada ou preservada.
- **Dado** lease expirado de worktree registrado.
- **Quando** cleanup é solicitado.
- **Então** retorna ação bounded para esse registro.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Contrato puro e determinístico de branch/worktree; nenhuma execução de Git, filesystem ou mutação externa.
