# Filesystem write tool

`FilesystemWriteTool` exige projeto correto, `PermissionDecision::Allowed`, operation key não vazia, path relativo e payload dentro do limite. A root é canonicalizada na construção; paths fora dela falham.

A escrita usa arquivo temporário no mesmo diretório e `rename`, evitando estado parcialmente escrito. Antes de substituir um arquivo existente, guarda snapshot bounded em memória. `rollback(operation_key)` remove arquivo novo ou restaura o snapshot anterior. Repetição da mesma operation key é no-op idempotente e não troca o conteúdo original da operação.

Este módulo não oferece delete/rename genérico, listagem, shell ou execução de conteúdo.
