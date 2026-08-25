# Agent invocation protocol

O contrato define request versionado com project/group/session/caller/callee,
trace, task bounded, context allowlist, budget token, depth e status.

A validação é provider-agnostic e fail-closed. `Pending`, `Completed`,
`Failed` e `Cancelled` são estados de dados; esta camada não executa
invocation, não escolhe provider e não transporta conteúdo implícito.
