# MCP transport abstraction

`agent-core::mcp_transport` define o contrato transport-neutral para envelopes MCP: versão, correlation ID, tipo de frame, capabilities e limite de tamanho.

O lifecycle usa estados cancelável/fechado idempotentes. A fila é bounded e retorna backpressure quando cheia ou inativa; reconnect é negado por padrão. O contrato não implementa stdio/HTTP, discovery, plugin loading, providers ou execução remota.
