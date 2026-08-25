# Agent-to-agent message rendering

`AgentMessageStore` aceita somente mensagens do project/session ativos, com IDs
válidos e identities conhecidas quando a allowlist é fornecida. Message IDs são
deduplicados.

`renderAgentMessage` mantém sender, receiver, invocation, round, trace e status,
mas sempre marca o texto como `untrusted-data` e fixa `actionAllowed: false`.
Markup é escapado; estados pending, result, error e terminated permanecem dados
visíveis.
