# Self-development PR

`agent-core::self_development_pr` prepara uma proposta de PR draft vinculada a candidate, issue, branch, base SHA, head SHA, tree e evidências de proposal, evaluation, regression e rollback.

A proposta é sempre `draft`, exige revisão e nunca é aprovada pelo contrato. Qualquer mudança de head ou tree torna a evidência `Stale`; duplicatas com identidade igual usam a mesma chave idempotente. Não há merge, release, activation, bypass ou acesso a secrets.
