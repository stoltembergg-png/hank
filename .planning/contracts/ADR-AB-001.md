# ADR-AB-001 — Core Rust independente de shell e adapters

- **Estado:** contrato normativo em fechamento; não é prova de implementação.
- **Decisão:** `agent-core` e os módulos de domínio não dependem de Tauri, UI, provider concreto, storage concreto ou processo externo. Tauri é shell/bridge/packaging; CLI e adapters fake exercem a mesma Application API.
- **Ownership:** Application API é owner dos use cases; Agent Runtime é owner do lifecycle de execução; ports pertencem ao core; adapters pertencem à infraestrutura/superfície correspondente.
- **Processo/lifecycle:** core é biblioteca reutilizável; shell/adapters iniciam e encerram recursos externos; nenhum adapter pode alterar regra de domínio.
- **Compatibilidade:** contratos de command/result/event são versionados; mudança incompatível exige ADR, fixture negativa e revalidação dos dependentes.
- **Proibições:** import de Tauri/UI no core; acesso direto de UI a storage/provider/tool; edge concreta substituindo port; regra de domínio em shell.
- **Prova exigida:** grafo versionado, matriz AB-001, validator de edges/ciclos e adapter fake/CLI com SHA/tree/policy.
- **Status:** `PARTIAL/NO_PROOF` até que a prova executável seja vinculada ao SHA atual.
