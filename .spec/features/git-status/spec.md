# Spec: Git status

> feature: git-status
> status: implementada

## Contexto

PR-106 expõe estado read-only de um repositório autorizado usando exclusivamente o process primitive.

## Histórias

### US-612 — Status Git read-only

Como runtime, quero consultar branch e mudanças de um repositório do projeto, para planejar ações sem mutar a árvore ou executar Git arbitrário.

#### AC-654 — Status autorizado e bounded

- **Dado** repository root válido, Git allowlisted e permission `Allowed`
- **Quando** consulto status
- **Então** retorno branch e entries `XY path` ordenados pelo Git, limitado por quantidade e sem modificar arquivos

#### AC-655 — Isolamento e fail-closed

- **Dado** projeto incorreto, permission pendente, repository inválido ou limite inválido
- **Quando** consulto
- **Então** a operação falha antes de retornar status

## Fora de escopo

- Commit, push, reset, checkout, hooks, configuração global, diff, shell e repos externos.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-621 | Git status porcelain v1 é estável para o primeiro parser. | confirmada | Parser usa `--porcelain=v1 -b`. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-612 | Deve incluir status de submodules? | respondida | Não neste card; serão tratados em contrato futuro. |
