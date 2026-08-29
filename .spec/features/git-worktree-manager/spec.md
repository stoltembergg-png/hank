# Spec: Git worktree manager

> feature: git-worktree-manager
> status: em-implementacao

## Contexto

PR-205 define worktrees isolados para tarefas de desenvolvimento. O primeiro
slice estabelece uma boundary pura em `agent-core`: registra a intenção de um
worktree, vincula task/workspace/owner, valida containment lexical e impede
colisões. A execução real de `git worktree` pertence a um adapter bounded em
`tool-core`; este domínio não executa processos, acessa filesystem ou altera
branches.

## História

### US-1302 — Worktree isolado e atribuível

Como runtime de desenvolvimento, quero reservar um worktree por task dentro da
raiz do workspace, para que worktrees concorrentes não compartilhem caminho,
branch ou ownership.

#### AC-1306 — Registro preserva task, workspace, owner e modo @spec:AC-1306

- **Dado** um request bounded com `task_id`, `workspace_id`, `owner_id`, raiz do workspace e path do worktree
- **Quando** registro o worktree
- **Então** a consulta retorna os mesmos vínculos e o modo `detached` ou `branch`, sem executar Git ou filesystem

#### AC-1307 — Registro idempotente e colisões explícitas @spec:AC-1307

- **Dado** um request já registrado
- **Quando** repito exatamente o mesmo request ou tento reutilizar task/path/branch com dados diferentes
- **Então** o request idêntico retorna o mesmo registro e a colisão falha sem substituir o owner original

#### AC-1308 — Path containment é fail-closed @spec:AC-1308

- **Dado** um path relativo, traversal ou path fora da raiz do workspace
- **Quando** tento registrar o worktree
- **Então** recebo `DomainError::Validation` antes de qualquer mutação do registry

#### AC-1309 — Adapter materializa request com argv seguro @spec:AC-1309

- **Dado** um request válido do registry e uma decisão `Allowed`
- **Quando** o adapter executa `add` no repository autorizado
- **Então** usa `git worktree add` sem shell, limita output, rejeita project/path/permission incompatíveis antes da execução e cria somente o worktree dentro do workspace configurado

#### AC-1310 — List e remove são bounded e fail-closed @spec:AC-1310

- **Dado** um repository autorizado
- **Quando** consulto `list` ou removo um worktree pelo path validado
- **Então** `list --porcelain` é parseado em registros estruturados e `remove` não usa force; output truncado, formato inválido, projeto estranho ou falha Git não são tratados como sucesso

## Fora de escopo

- Acesso direto a filesystem ou execução de Git pela camada de domínio
- Checkout, commit, push, merge, branch policy e credentials
- Persistência, restart recovery e orphan recovery
- Comandos Git diferentes de `add`, `list --porcelain` e `remove` sem force

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-1303 | O adapter Git receberá argv estruturado e um root autorizado. | confirmada | O domínio expõe request validado; `tool-core` será responsável por `ProcessSpec`. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-626 | Branches devem ser validadas por uma política própria? | respondida | Não neste slice; nomes são bounded e sem separadores perigosos. A política de branch é PR-206. |
