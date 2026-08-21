# Spec: Git diff

> feature: git-diff
> status: implementada

## Contexto

PR-107 expõe diff read-only bounded para revisão, com modos staged/unstaged/path, redaction e isolamento de repository.

## Histórias

### US-613 — Diff seguro para revisão

Como runtime, quero consultar diff de um repositório autorizado sem aplicar alterações, para revisar mudanças como conteúdo não confiável.

#### AC-656 — Diff autorizado e sem mutação

- **Dado** repository válido e permission `Allowed`
- **Quando** solicito diff unstaged
- **Então** recebo diff bounded com trace e a árvore não é alterada

#### AC-657 — Modos, redaction e truncamento

- **Dado** mudanças staged ou path autorizado contendo secret/prompt text
- **Quando** solicito diff
- **Então** staged/path funcionam, conteúdo sensível é redigido e limite marca truncamento

#### AC-658 — Isolamento e rejeições

- **Dado** projeto incorreto, permission pendente, path traversal ou limite inválido
- **Quando** solicito diff
- **Então** falha antes de executar Git

## Fora de escopo

- Apply patch, commit, checkout, merge, push, publicação e execução de conteúdo.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-623 | `git diff --no-ext-diff --no-textconv` evita hooks/textconv arbitrários. | confirmada | Flags são sempre adicionadas ao argv. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-613 | Diff binary precisa de representação própria? | respondida | Não neste card; bytes aparecem como metadata Git e output continua bounded. |
