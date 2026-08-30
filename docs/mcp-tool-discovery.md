# MCP tool discovery

`tool-core::mcp_discovery` processa manifests MCP somente após transport e server authorization. Valida revision, capabilities, nomes, schemas, quantidade e tamanho total, ordena entries deterministically e as coloca em `Pending`.

Discovery não executa tools nem ativa entries. Duplicatas, capabilities não autorizadas, manifests oversized e revisões stale são rejeitados. Refresh mantém `execution_enabled=false` e não amplia permissões.
