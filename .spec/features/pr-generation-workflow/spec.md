# Spec: PR generation workflow

> feature: pr-generation-workflow
> status: auditada

## Contexto

PR-213 define o contrato bounded para transformar um handoff validado do coding
agent em uma proposta de PR draft. A crate `agent-core` somente valida dados e
produz um plano declarativo; adapters externos cuidam de GitHub e persistência.

## Boundary e não-escopo

- O profile exige mapping `Active`, task/repository/worktree/branch e head/tree
  SHA exatos, policy revision, evidência e campos de PR completos.
- O domínio não acessa Git, filesystem, rede, GitHub, credentials, processos,
  provider ou secrets.
- O domínio não cria/atualiza PRs, aprova, faz merge, publica release, altera
  branch protegida ou concede token/capability.
- Handoff, diff, logs e metadata são dados não confiáveis; texto instruction-like
  é rejeitado e nunca é interpretado como comando.

## Histórias

### US-1337 — Draft handoff bounded

Como coding agent, quero produzir um handoff completo e vinculado ao mapping
para que um adapter possa propor uma PR draft reproduzível.

#### AC-1337 — Identidade e contrato

- **Dado** mapping ativo e handoff completo com SHA/tree/policy/evidence exatos.
- **Quando** o handoff é avaliado.
- **Então** um plano draft-only válido é produzido.
- **Dado** mapping inativo, branch/task/repository/worktree/SHA/tree/policy
  divergente, campo obrigatório ausente ou payload oversized.
- **Quando** o handoff é avaliado.
- **Então** falha fechado sem plano publicável.

### US-1338 — Idempotência e permissões

Como sistema de integração, quero distinguir criação de atualização pelo
fingerprint bounded sem conceder merge ou publicação irrestrita.

#### AC-1338 — Draft-only

- **Dado** fingerprint e idempotency key válidos.
- **Quando** o plano é construído.
- **Então** ele declara `CreateDraft` ou `UpdateDraft`, preserva identidade e
  retorna `can_merge=false` e `can_publish=false`.
- **Dado** tentativa de merge, publicação, branch protegida ou credencial.
- **Quando** o handoff é validado.
- **Então** é rejeitada antes de qualquer efeito.

### US-1339 — Metadata não confiável

Como sistema de segurança, quero impedir que texto do handoff altere autoridade
ou seja tratado como instrução executável.

#### AC-1339 — Hostile metadata

- **Dado** description, path, artifact ou risk instruction-like, com traversal,
  controle ou secret-like value.
- **Quando** incorporado ao handoff.
- **Então** é rejeitado como dado inválido, sem execução, approval ou capability.

## Definition of Done

Contrato, testes positivos/negativos, documentação e verify ONP passam; adapter
externo e publicação GitHub permanecem fora do escopo.
