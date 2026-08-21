# Git diff tool

`GitDiffTool` executa somente `git diff --no-ext-diff --no-textconv --unified=3`, adicionando `--cached` para staged ou `-- path` para path relativo autorizado. Repository e projeto são confinados e permission/timeout/output são herdados do process primitive.

O resultado é redigido para linhas contendo `secret`, `token`, `password` ou `api_key`; caracteres de controle são removidos e o limite acrescenta `[truncated]`. O diff é dado não confiável: este módulo não aplica patches, não comita e não interpreta instruções.
