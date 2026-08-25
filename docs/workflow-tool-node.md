# ToolNode adapter

`agent_runtime::tool_node::ToolNodeAdapter` é a fronteira bounded entre workflow e Tool Runtime.

- resolve somente ferramentas ativas no `ToolRegistry` e no project scope;
- valida `ToolRequest`, input schema e `PermissionEvaluator` antes do handler;
- rejeita permission denied, capability mismatch, unknown e oversized sem execução;
- aplica timeout e verifica cancellation antes/depois do handler;
- preserva `operation_key`, `trace_id` e `ToolResponse`;
- mantém cache bounded de outcomes para duplicate operation idempotente;
- não cria shell, ferramenta, sandbox ou bypass de confirmação.
