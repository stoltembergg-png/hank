# Filesystem read tool

`FilesystemReadTool` recebe roots já autorizadas pelo projeto e as canonicaliza na construção. A operação aceita somente path relativo, exige `ProjectId` igual ao projeto da tool e uma decisão `PermissionDecision::Allowed`.

Antes da leitura, o path é canonicalizado e comparado com cada root. Traversal, path absoluto, projeto diferente, decisão pendente/negada e symlink que escape da root falham fechadamente. O resultado é UTF-8 estrito, limitado por bytes, com path lógico, trace, quantidade lida e flag de truncamento.

O módulo não escreve, remove, lista diretórios, executa processos, descobre secrets ou interpreta conteúdo. Prompt injection dentro de um arquivo continua sendo apenas conteúdo retornado ao consumidor.
