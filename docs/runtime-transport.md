# Runtime transport

`agent-protocol` define o contrato transport-neutral para runtime remoto: envelope versionado, conexão/sessão/correlação bounded, capabilities declarativas, frame máximo de 64 KiB e lifecycle de sessão com fila limitada, cancelamento e fechamento idempotentes.

Reconnect, autenticação, sockets, daemon, WebSocket, dispatch e execução remota ficam fora desta PR. O contrato somente valida e transporta identidade; não concede autoridade.
