# AgentGroup entity

`AgentGroup` é uma entidade de domínio versionada e não executável. Ela
carrega identidade de projeto, owner, membros e seus bindings de projeto,
moderador, limites de rounds/depth, budget, referências de contexto não
confiáveis, lifecycle, versão pinned e trace.

Contexto compartilhado só aceita referências bounded `project://`; nenhum
conteúdo bruto é carregado pela entidade. A entidade não invoca agentes,
não acessa rede/filesystem/secrets e não mantém estado global.
