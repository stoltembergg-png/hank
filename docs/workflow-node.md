# Workflow node schema

`workflow-core` agora define `WorkflowNode` como contrato declarativo versionado.
Os oito tipos iniciais possuem enum fechado; input/output JSON, identidade,
timeout, retry/cancel policy e capabilities são bounded e validados antes de
qualquer composição.

A definição não executa handlers, não acessa storage e não contém secrets ou
shell configuration. Capability requirements começam vazios (deny-by-default)
e são apenas declarações para policy futura.
