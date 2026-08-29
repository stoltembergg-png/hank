# Reviewer agent profile

## Boundary

`agent-core::reviewer_profile` é um contrato puro para um agente que lê um
worktree ligado a um `TaskWorkspaceMapping` ativo. O profile não executa Git,
filesystem, rede, provider, processo ou secrets.

## Autorização

- A request deve coincidir exatamente com project, task, repository, worktree e
  branch do mapping ativo.
- Commit SHA e tree SHA devem ser hexadecimais de 40 ou 64 caracteres.
- A allowlist padrão contém somente `ReadFile`, `GitDiff`, `GitStatus`,
  `ReadChecks` e `ReadArtifact`.
- `WriteFile`, `write_attempt` e qualquer ferramenta mutating são rejeitados.
- Paths, sources, findings e evidence têm limites fixos; traversal, controles e
  payloads oversized falham closed.

## Evidence e handoff

`ReviewerEvidence` carrega somente kind, source, SHA/tree, status e digest.
`Passed`/`Failed` exigem digest SHA-256; `Missing`, `Skipped`, `NoRun`,
`Malformed` e `Stale` nunca podem produzir revisão completa. A validação do
report compara cada evidence ao SHA/tree do report e ao mapping.

`ReviewerFinding` pode ser `Observed` ou `Unknown`. Conteúdo de findings é
tratado como dado não confiável e não é interpretado como instrução.

`ReviewerReport` é sempre `ReviewerAuthority::Advisory`; seus métodos
`can_approve()` e `can_merge()` retornam `false` sem exceção. O relatório não
contém campos de aprovação, Ruleset, CODEOWNERS, secrets ou mutação.

## Escopo futuro

Execução real de ferramentas, integração com Git/CI/GitHub, armazenamento,
review UI e promoção de findings permanecem fora deste contrato e exigem cards
posteriores com authority independente.
