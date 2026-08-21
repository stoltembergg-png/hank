# Git status tool

`GitStatusTool` recebe project ID, repository root, executável Git allowlisted e limite de entries. Executa somente `git status --porcelain=v1 -b --untracked-files=normal` pelo process primitive, com `GIT_OPTIONAL_LOCKS=0`, stdin nulo, timeout e output bounded.

O resultado expõe branch e entries com os dois status porcelain e path lógico. A tool não chama shell, hooks, commit, push, reset ou checkout; permission/project/root inválidos falham antes do processo.
