# Tool plugins

`tool-core` expõe `ToolPluginAdapter<T>` sobre o contrato `Tool`. O wrapper vincula `plugin_id` e digest, exige aprovação explícita e delega somente após a validação existente de nome, versão, capability e decisão de policy.

Input/output, timeout, contexto e observabilidade permanecem sob `ToolRequest`/`ToolResponse`. A PR não adiciona shell, filesystem, rede, secrets, sandbox bypass, loader ou implementação de tool específica.
