# MCP settings UI

A tela MCP usa API typed para validar endpoints HTTPS, capabilities permitidas e revogação por escopo explícito. Textos de server/tool são renderizados como texto React, sem HTML interpretável.

Tools descobertas permanecem `Pending` ou `Disabled` e não possuem console de execução. O estado não persiste secrets nem acessa SQLite diretamente; respostas stale não podem restaurar grants.
