# Desktop project lifecycle bridge

A bridge Tauri dedicada expõe `create_project`, `list_projects`, `get_project`,
`update_project` e `archive_project`. Cada command recebe DTOs bounded, chama o
application service correspondente e retorna um DTO frontend explícito.

O state usa o mesmo `SqliteStorage` criado no boot, após migrations. A bridge não
contém regras de domínio nem escreve SQL. IDs, status, timestamps, settings,
paginação, correlation IDs e erros são convertidos conscientemente.

O client frontend não cria projects sintéticos quando IPC está ausente: retorna o
erro tipado `PROJECT_BRIDGE_UNAVAILABLE`.
