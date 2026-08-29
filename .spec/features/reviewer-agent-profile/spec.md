# Spec: Reviewer agent profile

> feature: reviewer-agent-profile
> status: auditada

## Contexto

PR-209 define um reviewer read-only que analisa diff, testes e artefatos de uma tarefa sem poder de mutação, aprovação ou merge. O reviewer consome somente um `TaskWorkspaceMapping` ativo e deve vincular cada resultado ao commit e à árvore exatos.

### US-1325 — Reviewer read-only scoped

#### AC-1325 — Scope and tool allowlist

- **Dado** um mapping ativo de projeto, task, repository, worktree e branch
- **Quando** o reviewer solicita uma ferramenta de leitura dentro do worktree
- **Então** o profile autoriza somente a ferramenta allowlisted e rejeita write, merge, rede arbitrária e caminho fora do escopo

### US-1326 — Exact evidence identity

#### AC-1326 — SHA/tree and evidence validation

- **Dado** um request com commit SHA e tree SHA
- **Quando** o reviewer produz findings e referências de evidence
- **Então** SHA/tree divergente, teste ausente, check skipped/no-run, artefato malformed ou digest inválido não podem ser classificados como revisão completa

### US-1327 — Advisory-only handoff

#### AC-1327 — Non-authority output

- **Dado** um relatório com findings observados ou desconhecidos
- **Quando** o handoff é materializado
- **Então** ele permanece bounded, advisory e proposal-only, sem métodos ou campos que concedam aprovação, alteração de gates, CODEOWNERS, Ruleset, secrets ou merge

## Envelope de segurança

- O módulo vive em `agent-core` e não importa Git, filesystem, rede, provider, processo, SQLite ou credenciais.
- O mapping deve estar `Active`; identidade de projeto/task/repository/worktree/branch deve coincidir exatamente.
- Tools permitidas são somente leitura e bounded; `WriteFile` e qualquer mutação são negados por padrão.
- SHA, tree SHA, digests, paths, sources, findings e evidências possuem limites e rejeitam formato inválido.
- Conteúdo de diff, artefato, finding e relatório é dado não confiável; nunca é interpretado como comando ou policy.
- Ausência, skip, no-run, stale, malformed ou digest inválido produz estado explícito não aprovável.
- O reviewer não é autoridade de aprovação: não haverá `approve`, `merge`, `set_required_check` ou mutação equivalente.

## Observabilidade

Relatórios carregam somente IDs de escopo, SHA/tree, policy revision, status, contagens e digests redigidos. Payload bruto de artefato e credenciais não entram no contrato.

## Rollback

O slice é aditivo e isolado em `agent-core`; revert do módulo, testes, docs e verificação ONP remove o profile sem migration ou estado externo.

## Suposições

- `TaskWorkspaceMapping` ativo é a fonte de verdade para a associação task→worktree→branch.
- SHA de commit/tree usa hexadecimal Git de 40 ou 64 caracteres; digest de artefato usa SHA-256 hexadecimal de 64 caracteres.

## Perguntas em aberto

Nenhuma.
