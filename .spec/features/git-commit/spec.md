# Spec: Git commit

> feature: git-commit
> status: implementada

## Contexto

PR-108 expõe commit explícito, autorizado, reversível e bounded sobre o process primitive, sem push/force push/reset/amend/hook arbitrário.

## Histórias

### US-614 — Commit mutação segura

Como runtime, quero commitar paths selecionados com mensagem validada, preflight, autorização explícita, operation key dedupe e rollback documentado, para mutar o repositório do projeto de forma auditável e reversível.

#### AC-659 — Commit autorizado com preflight e hash

- **Dado** repository válido, paths staged, permission `Allowed` com confirmação, operation key e message válida
- **Quando** solicito commit
- **Então** preflight status valida paths, commit executa, recebo hash e a árvore reflete a mutação

#### AC-660 — Isolamento, rejeições e validações

- **Dado** projeto incorreto, permission pendente, path traversal, message vazia, operation key ausente ou limite inválido
- **Quando** solicito commit
- **Então** falha antes de mutar o repositório

#### AC-661 — Validação de paths contra status

- **Dado** paths mistos (staged e unstaged)
- **Quando** solicito commit
- **Então** apenas paths presentes no status (staged ou unstaged) são aceitos; unstaged sem stage falha

## Fora de escopo

- Push, force push, reset, amend automático, hooks arbitrários, assinatura de credencial, alteração de repo externo.
- Rollback automático: documentado via procedimento de `git revert` operacional.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-624 | `git commit -m` com paths explícitos é atômico por repo. | confirmada | ProcessSpec com `allowed_programs`/`allowed_roots` isola execução. |
| ASM-625 | Author identity via `-c user.name/email` não persiste config global. | confirmada | Flags `-c` são por-invocação; não escrevem `.git/config`. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-614 | Commit signing (GPG/SSH) deve ser suportado? | respondida | Não neste card; requer credential storage fora de escopo. |