# Self-development branch

`agent-core::self_development_branch` define o mapping puro entre issue, candidate, versão, base SHA, policy e branch. A criação real fica fora do domínio e deve usar adapter com root allowlist, argv explícito, lease e timeout.

Issue ausente, policy negada, branch protegida e root não autorizado são bloqueados. O mapping é idempotente por chave determinística. Cleanup de registro expirado é uma decisão bounded; worktree desconhecido é sempre preservado.
