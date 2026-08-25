# Invocation graph

`InvocationGraph` registra requests validados com parent opcional, conserva
project scope, impõe limite de nós e fornece cancelamento idempotente.

O grafo é apenas estado bounded para preflight. Não agenda, executa, chama
provider/worker nem substitui as gates de cycle/depth das próximas fatias.
