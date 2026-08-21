# Directory list tool

`DirectoryListTool` canonicaliza roots autorizadas e retorna somente `name`, `kind` e `size_bytes`. A saída é ordenada por nome, limitada por `max_entries` e marca truncamento.

Filtros opcionais de prefix/suffix são bounded e determinísticos. Dotfiles ficam ocultos por padrão. Projeto, permission, path traversal, filtros inválidos e symlink apontando fora da root falham fechadamente. O módulo não lê conteúdo, não executa processos e não modifica filesystem.
