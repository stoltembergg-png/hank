# AgentGroup membership

Membership é representada por `agent_id`, `project_id` e role bounded dentro
do snapshot do grupo. Apenas owner ou moderator pode mutar; o binding do agente
deve ser o mesmo projeto do grupo; duplicatas, grupos arquivados e snapshots
inconsistentes falham fechados.

As operações são puras e não invocam agents, providers, tools ou sessões.
