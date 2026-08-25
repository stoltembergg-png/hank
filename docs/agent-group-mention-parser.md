# AgentGroup mention parser

O parser reconhece somente referências explícitas `@agent:<typed-id>` contra o
snapshot de membership fornecido. Targets são deduplicados e project-scoped;
input, target desconhecido, cross-project e fan-out excessivo falham fechados.

O resultado é uma referência de dados. `invocation_requested` permanece falso:
resolver menção nunca chama agent, tool, provider, rede ou persistence.
