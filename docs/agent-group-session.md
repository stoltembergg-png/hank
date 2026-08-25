# AgentGroup session

`AgentGroupSession` captura project/group/trace e congela o snapshot de
membership no momento da criação. Mantém rounds, depth, budget, tokens usados
e referências de contexto sem armazenar conteúdo bruto.

A sessão é uma máquina de estados bounded: `Created → Active → Closed` ou
`Cancelled`. Cancelamento é terminal e idempotente. Esta fatia não executa
agents, tools, providers, delegation ou streaming.
