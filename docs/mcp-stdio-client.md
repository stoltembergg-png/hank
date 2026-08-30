# MCP stdio client

O módulo `tool-core::mcp_stdio` define o contrato bounded do adapter local: executable absoluto dentro da allowlist, argv explícito sem metacaracteres de shell, limite de output e política de restart limitada.

O lifecycle expõe cancelamento, crash, timeout e restart bounded. Esta fatia não faz discovery, não executa shell arbitrário, não acessa secrets e não reivindica a implementação concreta do spawn/stdio.
