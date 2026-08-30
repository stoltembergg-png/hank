# Self-development issue handoff

`agent-core::self_development_issue` produz somente um payload bounded para handoff humano. O payload vincula candidate, evidence, repository, SHA, tree, policy, decisão, risco e próximo gate, com chave idempotente determinística.

Texto hostil é escapado ou redacted; `NO_GO` permanece explícito. Policy negada bloqueia a criação do payload. Não há publicação, mutação de código/branch/PR, acesso a segredo ou cross-posting.
