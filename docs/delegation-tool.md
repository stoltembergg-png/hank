# Delegation tool

A delegation tool request builder valida caller e callee contra o snapshot da
sessão, herda project/group/session/trace, valida task/context/budget pelo
invocation protocol e retorna somente estado `Pending`.

`PendingDelegationLedger` é um boundary explícito para dedupe e cancelamento.
Não há worker call, provider, transport, invocation graph ou execução nesta
fatia.
