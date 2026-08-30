# MCP HTTP client

`tool-core::mcp_http` adiciona a política MCP específica sobre o cliente HTTP existente: HTTPS por padrão, HTTP apenas com policy explícita, rejeição de credentials na URL, limites bounded e cancelamento terminal.

Retry é permitido apenas para métodos idempotentes ou POST com idempotency key, sempre limitado por contador. O contrato não armazena credenciais, não permite redirect/policy implícitos e não depende de internet pública nos testes.
