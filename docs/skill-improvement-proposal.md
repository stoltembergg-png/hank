# Skill improvement proposal

`agent-core::skill_improvement_proposal` cria somente uma proposal versionada e bounded. Ela preserva a versão ativa, referencia candidate/observation/policy, declara arquivos, capabilities e testes, e calcula fingerprint determinístico.

Paths ocultos, traversal, conteúdo secret-like, ausência de provenance/testes e diffs fora do limite são rejeitados. A proposal não instala, executa ou ativa scripts e não altera a skill ativa; avaliação e aprovação permanecem externas.
